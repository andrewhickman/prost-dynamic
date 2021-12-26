mod error;
mod service;
mod ty;

pub use self::{
    error::DescriptorError,
    service::{MethodDescriptor, ServiceDescriptor},
};

use std::{fmt, sync::Arc};

use prost_types::FileDescriptorSet;

use self::service::ServiceDescriptorInner;

pub(crate) const MAP_ENTRY_KEY_TAG: u32 = 1;
pub(crate) const MAP_ENTRY_VALUE_TAG: u32 = 2;

/// A wrapper around a [`FileDescriptorSet`], which provides convenient APIs for the
/// protobuf message definitions.
///
/// This type is immutable once constructed and uses reference counting internally, so it is
/// cheap to clone.
#[derive(Clone)]
pub struct FileDescriptor {
    inner: Arc<FileDescriptorInner>,
}

struct FileDescriptorInner {
    raw: FileDescriptorSet,
    type_map: ty::TypeMap,
    services: Vec<ServiceDescriptorInner>,
}

/// A protobuf message definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Descriptor {
    file_set: FileDescriptor,
    ty: ty::TypeId,
}

/// A protobuf message definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDescriptor {
    message: Descriptor,
    field: u32,
}

/// The type of a protobuf message field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldDescriptorKind {
    Double,
    Float,
    Int32,
    Int64,
    Uint32,
    Uint64,
    Sint32,
    Sint64,
    Fixed32,
    Fixed64,
    Sfixed32,
    Sfixed64,
    Bool,
    String,
    Bytes,
    Message(Descriptor),
    Enum(EnumDescriptor),
}

/// Cardinality determines whether a field is optional, required, or repeated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Cardinality {
    /// The field appears zero or one times.
    Optional,
    /// The field appears exactly one time. This cardinality is invalid with Proto3.
    Required,
    /// The field appears zero or more times.
    Repeated,
}

/// A protobuf enum descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDescriptor {
    file_set: FileDescriptor,
    ty: ty::TypeId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumValueDescriptor {
    parent: EnumDescriptor,
    number: i32,
}

impl FileDescriptor {
    /// Create a [`FileDescriptor`] from a [`FileDescriptorSet`].
    ///
    /// This method may return an error if `file_descriptor_set` is invalid, for example
    /// it contains references to types not in the set. If `file_descriptor_set` was created by
    /// the protobuf compiler, these error cases should never occur.
    pub fn new(file_descriptor_set: FileDescriptorSet) -> Result<Self, DescriptorError> {
        let inner = FileDescriptor::from_raw(file_descriptor_set)?;
        Ok(FileDescriptor {
            inner: Arc::new(inner),
        })
    }

    fn from_raw(raw: FileDescriptorSet) -> Result<FileDescriptorInner, DescriptorError> {
        let mut type_map = ty::TypeMap::new();
        type_map.add_files(&raw)?;
        type_map.shrink_to_fit();
        let type_map_ref = &type_map;

        let services = raw
            .file
            .iter()
            .flat_map(|raw_file| {
                raw_file.service.iter().map(move |raw_service| {
                    ServiceDescriptorInner::from_raw(raw_file, raw_service, type_map_ref)
                })
            })
            .collect::<Result<_, _>>()?;

        Ok(FileDescriptorInner {
            raw,
            type_map,
            services,
        })
    }

    /// Gets a reference the [`FileDescriptorSet`] wrapped by this [`FileDescriptor`].
    pub fn file_descriptor_set(&self) -> &FileDescriptorSet {
        &self.inner.raw
    }

    /// Gets an iterator over the services defined in these protobuf files.
    pub fn services(&self) -> impl ExactSizeIterator<Item = ServiceDescriptor> + '_ {
        (0..self.inner.services.len()).map(move |index| ServiceDescriptor::new(self.clone(), index))
    }

    /// Gets a protobuf message by its fully qualified name, for example `.PackageName.MessageName`.
    pub fn get_message_by_name(&self, name: &str) -> Option<Descriptor> {
        let ty = self.inner.type_map.get_by_name(name).ok()?;
        Some(Descriptor {
            file_set: self.clone(),
            ty,
        })
    }
}

impl fmt::Debug for FileDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileDescriptor")
            .field("services", &self.inner.services)
            .finish_non_exhaustive()
    }
}

impl PartialEq for FileDescriptor {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for FileDescriptor {}

impl Descriptor {
    /// Gets a reference to the [`FileDescriptor`] this message is defined in.
    pub fn file_descriptor(&self) -> &FileDescriptor {
        &self.file_set
    }

    pub fn fields(&self) -> impl ExactSizeIterator<Item = FieldDescriptor> + '_ {
        self.message_ty()
            .fields
            .keys()
            .map(move |&field| FieldDescriptor {
                message: self.clone(),
                field,
            })
    }

    pub fn get_field(&self, number: u32) -> Option<FieldDescriptor> {
        if self.message_ty().fields.contains_key(&number) {
            Some(FieldDescriptor {
                message: self.clone(),
                field: number,
            })
        } else {
            None
        }
    }

    pub fn get_field_by_name(&self, name: &str) -> Option<FieldDescriptor> {
        self.message_ty()
            .field_names
            .get(name)
            .map(|&number| FieldDescriptor {
                message: self.clone(),
                field: number,
            })
    }

    pub fn is_map_entry(&self) -> bool {
        self.message_ty().is_map_entry
    }

    fn message_ty(&self) -> &ty::Message {
        self.file_set.inner.type_map[self.ty]
            .as_message()
            .expect("descriptor is not a message type")
    }
}

impl FieldDescriptor {
    pub fn tag(&self) -> u32 {
        self.field
    }

    pub fn name(&self) -> &str {
        &self.message_field_ty().name
    }

    pub fn json_name(&self) -> &str {
        &self.message_field_ty().json_name
    }

    pub fn is_group(&self) -> bool {
        self.message_field_ty().is_group
    }

    pub fn is_list(&self) -> bool {
        self.cardinality() == Cardinality::Repeated && !self.is_map()
    }

    pub fn is_packed(&self) -> bool {
        self.message_field_ty().is_packed
    }

    pub fn is_map(&self) -> bool {
        self.cardinality() == Cardinality::Repeated
            && match self.kind() {
                FieldDescriptorKind::Message(message) => message.is_map_entry(),
                _ => false,
            }
    }

    pub fn cardinality(&self) -> Cardinality {
        self.message_field_ty().cardinality
    }

    pub fn supports_presence(&self) -> bool {
        self.message_field_ty().supports_presence
    }

    pub fn kind(&self) -> FieldDescriptorKind {
        let ty = self.message_field_ty().ty;
        match &self.message.file_set.inner.type_map[ty] {
            ty::Type::Message(_) => FieldDescriptorKind::Message(Descriptor {
                file_set: self.message.file_set.clone(),
                ty,
            }),
            ty::Type::Enum(_) => FieldDescriptorKind::Enum(EnumDescriptor {
                file_set: self.message.file_set.clone(),
                ty,
            }),
            ty::Type::Scalar(scalar) => match scalar {
                ty::Scalar::Double => FieldDescriptorKind::Double,
                ty::Scalar::Float => FieldDescriptorKind::Float,
                ty::Scalar::Int32 => FieldDescriptorKind::Int32,
                ty::Scalar::Int64 => FieldDescriptorKind::Int64,
                ty::Scalar::Uint32 => FieldDescriptorKind::Uint32,
                ty::Scalar::Uint64 => FieldDescriptorKind::Uint64,
                ty::Scalar::Sint32 => FieldDescriptorKind::Sint32,
                ty::Scalar::Sint64 => FieldDescriptorKind::Sint64,
                ty::Scalar::Fixed32 => FieldDescriptorKind::Fixed32,
                ty::Scalar::Fixed64 => FieldDescriptorKind::Fixed64,
                ty::Scalar::Sfixed32 => FieldDescriptorKind::Sfixed32,
                ty::Scalar::Sfixed64 => FieldDescriptorKind::Sfixed64,
                ty::Scalar::Bool => FieldDescriptorKind::Bool,
                ty::Scalar::String => FieldDescriptorKind::String,
                ty::Scalar::Bytes => FieldDescriptorKind::Bytes,
            },
        }
    }

    pub(crate) fn default_value(&self) -> Option<&crate::Value> {
        self.message_field_ty().default_value.as_ref()
    }

    fn message_field_ty(&self) -> &ty::MessageField {
        &self.message.message_ty().fields[&self.field]
    }
}

impl FieldDescriptorKind {
    pub fn as_message(&self) -> Option<&Descriptor> {
        match self {
            FieldDescriptorKind::Message(desc) => Some(desc),
            _ => None,
        }
    }
}

impl EnumDescriptor {
    pub fn default_value(&self) -> EnumValueDescriptor {
        self.values().next().unwrap()
    }

    pub fn values(&self) -> impl ExactSizeIterator<Item = EnumValueDescriptor> + '_ {
        self.enum_ty()
            .values
            .iter()
            .map(move |v| EnumValueDescriptor {
                parent: self.clone(),
                number: v.number,
            })
    }

    fn enum_ty(&self) -> &ty::Enum {
        self.file_set.inner.type_map[self.ty].as_enum().unwrap()
    }
}

impl EnumValueDescriptor {
    pub fn number(&self) -> i32 {
        self.number
    }
}
