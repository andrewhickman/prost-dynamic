use std::{
    env, fs,
    path::{Path, PathBuf},
};

use insta::assert_yaml_snapshot;
use miette::JSONReportHandler;
use prost::Message;
use prost_reflect::{DescriptorError, DescriptorPool, DynamicMessage, ReflectMessage};
use prost_types::FileDescriptorSet;

fn test_data_dir() -> PathBuf {
    PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("tests/data")
}

fn read_file_descriptor_set(path: impl AsRef<Path>) -> DynamicMessage {
    let yaml_bytes = fs::read(test_data_dir().join(path)).unwrap();

    let deserializer = serde_yaml::Deserializer::from_slice(&yaml_bytes);
    DynamicMessage::deserialize(FileDescriptorSet::default().descriptor(), deserializer).unwrap()
}

fn check(name: &str, add_wkt: bool) -> Result<DescriptorPool, DescriptorError> {
    let input = read_file_descriptor_set(format!("{}.yml", name));
    let proto_bytes = input.encode_to_vec();

    let mut pool = if add_wkt {
        FileDescriptorSet::default()
            .descriptor()
            .parent_pool()
            .clone()
    } else {
        DescriptorPool::new()
    };
    pool.decode_file_descriptor_set(proto_bytes.as_slice())?;

    Ok(pool)
}

fn check_ok(name: &str, add_wkt: bool) {
    let pool = check(name, add_wkt).unwrap();
    let set_desc = pool
        .get_message_by_name("google.protobuf.FileDescriptorSet")
        .unwrap_or_else(|| FileDescriptorSet::default().descriptor());

    let mut actual = DynamicMessage::decode(set_desc, pool.encode_to_vec().as_slice()).unwrap();

    if add_wkt {
        actual
            .get_field_by_name_mut("file")
            .unwrap()
            .as_list_mut()
            .unwrap()
            .retain(|f| {
                !f.as_message()
                    .unwrap()
                    .get_field_by_name("package")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .starts_with("google.protobuf")
            });
    }

    assert_yaml_snapshot!(name, actual);
}

fn check_err(name: &str) {
    let actual_err = check(name, false).unwrap_err();
    let mut actual_json = String::new();
    JSONReportHandler::new()
        .render_report(&mut actual_json, &actual_err)
        .unwrap();
    let actual = serde_json::from_str::<serde_json::Value>(&actual_json).unwrap();

    assert_yaml_snapshot!(name, actual);
}

macro_rules! check_ok {
    ($name:ident) => {
        #[test]
        fn $name() {
            check_ok(stringify!($name), false);
        }
    };
    ($name:ident, add_wkt: true) => {
        #[test]
        fn $name() {
            check_ok(stringify!($name), true);
        }
    };
}

macro_rules! check_err {
    ($name:ident) => {
        #[test]
        fn $name() {
            check_err(stringify!($name));
        }
    };
}

check_err!(name_conflict_in_imported_files);
check_err!(name_conflict_with_import);
check_err!(name_conflict_package1);
check_err!(name_conflict_package2);
check_ok!(name_conflict_package3);
check_err!(name_conflict_field_camel_case1);
check_err!(name_conflict_field_camel_case2);
check_ok!(name_conflict_field_camel_case3);
check_err!(name_conflict1);
check_err!(name_conflict2);
check_err!(name_conflict3);
check_err!(invalid_message_number1);
check_err!(invalid_message_number2);
check_err!(generate_map_entry_message_name_conflict);
check_err!(generate_group_message_name_conflict);
check_err!(generate_synthetic_oneof_name_conflict);
check_err!(invalid_service_type1);
check_err!(invalid_service_type2);
check_err!(invalid_service_type3);
check_err!(name_resolution1);
check_err!(name_resolution2);
check_err!(name_collision1);
check_err!(name_collision2);
check_err!(name_collision3);
check_err!(name_collision4);
check_err!(name_collision5);
check_err!(field_default_value1);
check_ok!(field_default_value2);
check_err!(enum_field_invalid_default1);
check_err!(enum_field_invalid_default2);
check_err!(enum_field_invalid_default3);
check_err!(enum_field_invalid_default4);
check_ok!(enum_field_invalid_default5);
check_err!(enum_field_invalid_default6);
check_ok!(enum_field_invalid_default7);
check_err!(enum_field_invalid_default8);
check_ok!(enum_field_invalid_default9);
check_err!(field_default_invalid_type1);
check_err!(field_default_invalid_type2);
check_err!(field_default_invalid_type3);
check_err!(message_field_duplicate_number1);
check_err!(message_field_duplicate_number2);
check_err!(message_reserved_range_overlap_with_field1);
check_err!(message_reserved_range_overlap_with_field2);
check_ok!(message_reserved_range_message_set1);
check_ok!(message_reserved_range_message_set2);
check_ok!(extend_group_field);
check_err!(extend_field_number_not_in_extensions1);
check_err!(extend_field_number_not_in_extensions2);
check_ok!(oneof_group_field);
check_err!(enum_reserved_range_overlap_with_value1);
check_err!(enum_reserved_range_overlap_with_value2);
check_err!(enum_reserved_range_overlap_with_value3);
check_err!(enum_duplicate_number1);
check_err!(enum_duplicate_number2);
check_ok!(enum_duplicate_number3);
check_ok!(enum_default1);
check_err!(enum_default2);
check_ok!(enum_default3);
check_err!(option_unknown_field);
check_err!(option_unknown_extension);
check_err!(option_already_set);
check_ok!(option_map_entry_set_explicitly);
check_ok!(option_resolution1, add_wkt: true);
check_ok!(option_resolution2, add_wkt: true);
check_ok!(option_resolution3, add_wkt: true);
check_ok!(option_resolution4, add_wkt: true);
check_ok!(option_resolution5, add_wkt: true);
// TODO need option name resolution
// check_ok!(option_resolution6, add_wkt: true);
check_ok!(option_resolution7, add_wkt: true);
check_ok!(option_resolution8, add_wkt: true);
check_ok!(option_resolution9, add_wkt: true);
check_ok!(option_resolution10, add_wkt: true);
check_ok!(option_resolution11, add_wkt: true);


/*
syntax = "proto2";

import "google/protobuf/any.proto";
import "google/protobuf/descriptor.proto";

option (a) = {
    [type.googleapis.com/Foo]: { foo: "bar" }
};

extend google.protobuf.FileOptions {
    repeated google.protobuf.Any a = 1001;
}

message Foo {
    optional string foo = 1;
}

*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a) = {
    foo: <
    >
};

extend google.protobuf.FileOptions {
    repeated Foo a = 1001;
}

message Foo {
    optional Foo foo = 1;
}

*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a) = {
    foo <
    >
};

extend google.protobuf.FileOptions {
    repeated Foo a = 1001;
}

message Foo {
    optional Foo foo = 1;
}

*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a) = {
    foo: [1, 2, 3]
};

extend google.protobuf.FileOptions {
    repeated Foo a = 1001;
}

message Foo {
    repeated int32 foo = 1;
}

*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a) = {
    foo: 1
    foo: 2
    foo: 3
};

extend google.protobuf.FileOptions {
    repeated Foo a = 1001;
}

message Foo {
    repeated int32 foo = 1;
}

*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a) = {
    Foo {
    }
};

extend google.protobuf.FileOptions {
    repeated Foo a = 1001;
}

message Foo {
    optional group Foo = 1 {};
}

*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a) = {
    Foo : {
    }
};

extend google.protobuf.FileOptions {
    repeated Foo a = 1001;
}

message Foo {
    optional group Foo = 1 {};
}

*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a) = {
    foo: 1
};
option (a).foo = 2;
option (a).bar = 2;

extend google.protobuf.FileOptions {
    optional Foo a = 1001;
}

message Foo {
    repeated int32 foo = 1;
    optional int32 bar = 2;
}

*/

/*


foo.proto:8:8: Option field "(a)" is a repeated message. Repeated message options must be initialized using an aggregate value.

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a) = {
    foo: 1
};
option (a).foo = 2;

extend google.protobuf.FileOptions {
    repeated Foo a = 1001;
}

message Foo {
    repeated int32 foo = 1;
    optional int32 bar = 2;
}


*/

/*

syntax = "proto3";

package google.protobuf;

message FileOptions {
    optional string java_outer_classname = 1;
}

option java_outer_classname = "ClassName";

*/
