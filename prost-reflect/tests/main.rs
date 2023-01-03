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

fn check(name: &str) -> Result<DescriptorPool, DescriptorError> {
    let input = read_file_descriptor_set(format!("{}.yml", name));
    let proto_bytes = input.encode_to_vec();

    DescriptorPool::decode(proto_bytes.as_slice())
}

fn check_ok(name: &str) {
    let actual_bytes = check(name).unwrap().encode_to_vec();
    let actual = DynamicMessage::decode(
        FileDescriptorSet::default().descriptor(),
        actual_bytes.as_slice(),
    )
    .unwrap();

    assert_yaml_snapshot!(name, actual);
}

fn check_err(name: &str) {
    let actual_err = check(name).unwrap_err();
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
            check_ok(stringify!($name));
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

/*
#[test]
fn invalid_message_number() {
    assert_eq!(
        check_err("message Foo { optional int32 i = 19000; }"),
        vec![ReservedMessageNumber {
            span: Some(SourceSpan::from(33..38))
        }]
    );
    assert_eq!(
        check_err("message Foo { optional int32 i = 19999; }"),
        vec![ReservedMessageNumber {
            span: Some(SourceSpan::from(33..38))
        }]
    );
}

#[test]
fn generate_map_entry_message_name_conflict() {
    assert_eq!(
        check_err(
            "message Foo {\
                map<uint32, bytes> baz = 1;

                enum BazEntry {
                    ZERO = 0;
                }
            }"
        ),
        vec![DuplicateName(DuplicateNameError {
            name: "Foo.BazEntry".to_owned(),
            first: NameLocation::Unknown,
            second: NameLocation::Root(SourceSpan::from(63..71)),
        })]
    );
}

#[test]
fn generate_group_message_name_conflict() {
    assert_eq!(
        check_err(
            "\
            message Foo {\
                optional group Baz = 1 {}

                enum Baz {
                    ZERO = 0;
                }
            }"
        ),
        vec![DuplicateName(DuplicateNameError {
            name: "Foo.Baz".to_owned(),
            first: NameLocation::Root(SourceSpan::from(28..31)),
            second: NameLocation::Root(SourceSpan::from(61..64)),
        })],
    );
}

#[test]
fn generate_synthetic_oneof_name_conflict() {
    assert_eq!(
        check_err(
            "\
            syntax = 'proto3';

            message Foo {
                optional fixed64 val = 1;

                message _val {}
            }"
        ),
        vec![DuplicateName(DuplicateNameError {
            name: "Foo._val".to_owned(),
            first: NameLocation::Unknown,
            second: NameLocation::Root(SourceSpan::from(113..117)),
        })],
    );
}

#[test]
fn invalid_service_type() {
    // use enum/service/oneof etc
    assert_eq!(
        check_err(
            "\
            syntax = 'proto3';

            enum Enum {
                ZERO = 0;
            }
            message Message {}

            service Service {
                rpc rpc(.Enum) returns (.Message);
            }"
        ),
        vec![InvalidMethodTypeName {
            name: ".Enum".to_owned(),
            kind: "input",
            span: Some(SourceSpan::from(170..175)),
        }],
    );
    assert_eq!(
        check_err(
            "\
            syntax = 'proto3';

            enum Enum {
                ZERO = 0;
            }
            message Message {}

            service Service {
                rpc rpc(.Message) returns (.Enum);
            }"
        ),
        vec![InvalidMethodTypeName {
            name: ".Enum".to_owned(),
            kind: "output",
            span: Some(SourceSpan::from(189..194)),
        }],
    );
    assert_eq!(
        check_err(
            "\
            syntax = 'proto3';

            message Message {}

            service Service {
                rpc rpc(.Message) returns (.Service);
            }"
        ),
        vec![InvalidMethodTypeName {
            name: ".Service".to_owned(),
            kind: "output",
            span: Some(SourceSpan::from(125..133)),
        }],
    );
}

#[test]
fn name_resolution() {
    assert_eq!(
        check_with_imports(vec![
            ("dep.proto", "package foo.bar; message FooBar {}"),
            (
                "root.proto",
                r#"
                syntax = 'proto3';

                import "dep.proto";

                message Foo {
                    .foo.FooBar foobar = 1;
                }"#
            ),
        ])
        .unwrap_err(),
        vec![TypeNameNotFound {
            name: ".foo.FooBar".to_owned(),
            span: Some(SourceSpan::from(124..135)),
        }]
    );
    assert_eq!(
        check_with_imports(vec![
            ("dep.proto", "package foo.bar; message FooBar {}"),
            (
                "root.proto",
                r#"
                syntax = 'proto3';

                import "dep.proto";

                message Foo {
                    .FooBar foobar = 1;
                }"#
            ),
        ])
        .unwrap_err(),
        vec![TypeNameNotFound {
            name: ".FooBar".to_owned(),
            span: Some(SourceSpan::from(124..131)),
        }]
    );
}

#[test]
fn name_collision() {
    assert_eq!(
        check_err(
            "\
            message Message {}
            message Message {}
            "
        ),
        vec![DuplicateName(DuplicateNameError {
            name: "Message".to_owned(),
            first: NameLocation::Root(SourceSpan::from(8..15)),
            second: NameLocation::Root(SourceSpan::from(39..46)),
        })],
    );
    assert_eq!(
        check_err(
            "\
            message Message {}
            enum Message {
                ZERO = 1;
            }"
        ),
        vec![DuplicateName(DuplicateNameError {
            name: "Message".to_owned(),
            first: NameLocation::Root(SourceSpan::from(8..15)),
            second: NameLocation::Root(SourceSpan::from(36..43)),
        })],
    );
    assert_eq!(
        check_err(
            "\
            message Message {
                optional int32 foo = 1;

                enum foo {
                    ZERO = 1;
                }
            }"
        ),
        vec![DuplicateName(DuplicateNameError {
            name: "Message.foo".to_owned(),
            first: NameLocation::Root(SourceSpan::from(49..52)),
            second: NameLocation::Root(SourceSpan::from(80..83)),
        })],
    );
    assert_eq!(
        check_with_imports(vec![
            ("dep.proto", "package foo;"),
            ("root.proto", "import 'dep.proto'; message foo {}"),
        ])
        .unwrap_err(),
        vec![DuplicateName(DuplicateNameError {
            name: "foo".to_owned(),
            first: NameLocation::Import("dep.proto".to_owned()),
            second: NameLocation::Root(SourceSpan::from(28..31)),
        })],
    );
    assert_eq!(
        check_with_imports(vec![
            ("dep.proto", "package foo.bar;"),
            ("root.proto", "import 'dep.proto'; message foo {}"),
        ])
        .unwrap_err(),
        vec![DuplicateName(DuplicateNameError {
            name: "foo".to_owned(),
            first: NameLocation::Import("dep.proto".to_owned()),
            second: NameLocation::Root(SourceSpan::from(28..31)),
        })],
    );
}

#[test]
fn field_default_value() {
    assert_eq!(
        check_err(
            r#"
            message Message {
                optional Message foo = 1 [default = ""];
            }"#
        ),
        vec![InvalidDefault {
            kind: "message",
            span: Some(SourceSpan::from(83..85)),
        }],
    );
    assert_yaml_snapshot!(check_ok(
        r#"
            message Message {
                optional float default_float_exp = 23 [ default = 9e6];
                optional double default_double_exp = 24 [ default = 9e22];
            }
        "#
    ));
}

#[test]
fn enum_field_invalid_default() {
    assert_eq!(
        check_err(
            r#"
            message Message {
                optional Foo foo = 1 [default = ONE];
            }

            enum Foo {
                ZERO = 0;
            }"#
        ),
        vec![InvalidEnumValue {
            value_name: "ONE".to_owned(),
            enum_name: "Foo".to_owned(),
            span: Some(SourceSpan::from(79..82)),
            help: Some("possible values are 'ZERO'".to_owned())
        }],
    );
    assert_eq!(
        check_err(
            r#"
            message Message {
                optional Foo foo = 1 [default = ONE];
            }

            enum Foo {
                ZERO = 0;
                TWO = 2;
            }

            enum Bar {
                NONE = 0;
                ONE = 1;
            }"#
        ),
        vec![InvalidEnumValue {
            value_name: "ONE".to_owned(),
            enum_name: "Foo".to_owned(),
            span: Some(SourceSpan::from(79..82)),
            help: Some("possible values are 'TWO' and 'ZERO'".to_owned()),
        }],
    );
    assert_eq!(
        check_with_imports(vec![
            (
                "dep.proto",
                "
                package foo;
                enum Foo { ZERO = 1; }"
            ),
            (
                "root.proto",
                r#"
                import "dep.proto";

                message Bar {
                    optional foo.Foo foo = 1 [default = ONE];
                }"#
            )
        ])
        .unwrap_err(),
        vec![InvalidEnumValue {
            value_name: "ONE".to_owned(),
            enum_name: "foo.Foo".to_owned(),
            span: Some(SourceSpan::from(124..127)),
            help: Some("possible values are 'ZERO'".to_owned()),
        }],
    );
    assert_eq!(
        check_err(
            "
            package foo;

            message Message {
                optional Foo foo = 1 [default = ONE];
            }

            enum Foo {
                ZERO = 0;
            }"
        ),
        vec![InvalidEnumValue {
            value_name: "ONE".to_owned(),
            enum_name: "foo.Foo".to_owned(),
            span: Some(SourceSpan::from(105..108)),
            help: Some("possible values are 'ZERO'".to_owned()),
        }],
    );
    assert_yaml_snapshot!(check_ok(
        "
        package foo;

        message Message {
            optional Foo foo = 1 [default = ZERO];
        }

        enum Foo {
            ZERO = 0;
        }"
    ));
    assert_eq!(
        check_err(
            "
            package foo;

            message Message {
                optional Foo foo = 1 [default = ONE];

                enum Foo {
                    ZERO = 0;
                }
            }"
        ),
        vec![InvalidEnumValue {
            value_name: "ONE".to_owned(),
            enum_name: "foo.Message.Foo".to_owned(),
            span: Some(SourceSpan::from(105..108)),
            help: Some("possible values are 'ZERO'".to_owned()),
        }],
    );
    assert_yaml_snapshot!(check_ok(
        "
        package foo;

        message Message {
            optional Foo foo = 1 [default = ZERO];

            enum Foo {
                ZERO = 0;
            }
        }"
    ));
    assert_eq!(
        check_err(
            "
            package foo;

            message Message {
                optional Parent.Foo foo = 1 [default = ONE];
            }

            message Parent {
                enum Foo {
                    ZERO = 0;
                }
            }"
        ),
        vec![InvalidEnumValue {
            value_name: "ONE".to_owned(),
            enum_name: "foo.Parent.Foo".to_owned(),
            span: Some(SourceSpan::from(112..115)),
            help: Some("possible values are 'ZERO'".to_owned()),
        }],
    );
    assert_yaml_snapshot!(check_ok(
        "
        package foo;

        message Message {
            optional Parent.Foo foo = 1 [default = ZERO];
        }

        message Parent {
            enum Foo {
                ZERO = 0;
            }
        }"
    ));
}

#[test]
fn field_default_invalid_type() {
    assert_eq!(
        check_err(
            r#"
            message Message {
                optional Foo foo = 1 [default = 1];
            }

            enum Foo {
                ZERO = 0;
            }"#
        ),
        vec![ValueInvalidType {
            expected: "an enum value identifier".to_owned(),
            actual: "1".to_owned(),
            span: Some(SourceSpan::from(79..80)),
        }],
    );
    assert_eq!(
        check_err(
            r#"
            message Message {
                optional Foo foo = 1 [default = "ZERO"];
            }

            enum Foo {
                ZERO = 0;
            }"#
        ),
        vec![ValueInvalidType {
            expected: "an enum value identifier".to_owned(),
            actual: "\"ZERO\"".to_owned(),
            span: Some(SourceSpan::from(79..85)),
        }],
    );
    assert_eq!(
        check_err(
            r#"
            message Message {
                optional Foo foo = 1 [default = FALSE];
            }

            enum Foo {
                ZERO = 0;
            }"#
        ),
        vec![InvalidEnumValue {
            value_name: "FALSE".to_owned(),
            enum_name: "Foo".to_owned(),
            span: Some(SourceSpan::from(79..84)),
            help: Some("possible values are 'ZERO'".to_owned()),
        }],
    );
}

#[test]
fn message_field_duplicate_number() {
    assert_eq!(
        check_err(
            r#"message Message {
                optional int32 foo = 1;
                optional int32 bar = 1;
            }"#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::Field {
                name: "foo".to_owned(),
                number: 1
            },
            first_span: Some(SourceSpan::from(55..56)),
            second: resolve::NumberKind::Field {
                name: "bar".to_owned(),
                number: 1
            },
            second_span: Some(SourceSpan::from(95..96)),
        })],
    );
    assert_eq!(
        check_err(
            r#"message Message {
                message Nested {
                    optional int32 foo = 1;
                    optional int32 bar = 1;
                }
            }"#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::Field {
                name: "foo".to_owned(),
                number: 1
            },
            first_span: Some(SourceSpan::from(92..93)),
            second: resolve::NumberKind::Field {
                name: "bar".to_owned(),
                number: 1
            },
            second_span: Some(SourceSpan::from(136..137)),
        })],
    );
}

#[test]
fn message_reserved_range_invalid() {
    assert_eq!(
        check_err(
            r#"message Message {
                reserved 5 to 1;
            }"#
        ),
        vec![InvalidRange {
            span: Some(SourceSpan::from(43..49))
        }],
    );
    assert_eq!(
        check_err(
            r#"message Message {
                reserved 2 to 1;
            }"#
        ),
        vec![InvalidRange {
            span: Some(SourceSpan::from(43..49))
        }],
    );
    assert_yaml_snapshot!(check_ok(
        r#"message Message {
            reserved 1 to 1;
        }"#
    ));
}

#[test]
fn message_reserved_range_overlap() {
    assert_eq!(
        check_err(
            r#"message Message {
                reserved 1;
                reserved 1;
            }"#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::ReservedRange { start: 1, end: 1 },
            first_span: Some(SourceSpan::from(43..44)),
            second: resolve::NumberKind::ReservedRange { start: 1, end: 1 },
            second_span: Some(SourceSpan::from(71..72)),
        })],
    );
    assert_eq!(
        check_err(
            r#"message Message {
                reserved 1 to 3;
                reserved 2 to 4;
            }"#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::ReservedRange { start: 1, end: 3 },
            first_span: Some(SourceSpan::from(43..49)),
            second: resolve::NumberKind::ReservedRange { start: 2, end: 4 },
            second_span: Some(SourceSpan::from(76..82)),
        })],
    );
    assert_eq!(
        check_err(
            r#"message Message {
                reserved 1 to 3;
                extensions 3 to max;
            }"#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::ReservedRange { start: 1, end: 3 },
            first_span: Some(SourceSpan::from(43..49)),
            second: resolve::NumberKind::ExtensionRange {
                start: 3,
                end: 536870911,
            },
            second_span: Some(SourceSpan::from(78..86)),
        })],
    );
    assert_yaml_snapshot!(check_ok(
        r#"message Message {
            reserved 1;
            extensions 2 to 3;
            reserved 4 to max;
        }"#
    ));
}

#[test]
fn message_reserved_range_overlap_with_field() {
    assert_eq!(
        check_err(
            r#"message Message {
                optional int32 field = 2;
                reserved 2;
            }"#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::ReservedRange { start: 2, end: 2 },
            first_span: Some(SourceSpan::from(85..86)),
            second: resolve::NumberKind::Field {
                name: "field".to_owned(),
                number: 2,
            },
            second_span: Some(SourceSpan::from(57..58)),
        })],
    );
    assert_eq!(
        check_err(
            r#"message Message {
                optional int32 field = 2;
                extensions 1 to 5;
            }"#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::ExtensionRange { start: 1, end: 5 },
            first_span: Some(SourceSpan::from(87..93)),
            second: resolve::NumberKind::Field {
                name: "field".to_owned(),
                number: 2,
            },
            second_span: Some(SourceSpan::from(57..58)),
        })],
    );
}

#[test]
fn message_reserved_range_message_set() {
    assert_yaml_snapshot!(check_ok(
        r#"
        message Foo {
            reserved 1 to max;

            option message_set_wire_format = true;
        }
        "#
    ));
    assert_yaml_snapshot!(check_ok(
        r#"
        message Foo {
            extensions 1 to max;

            option message_set_wire_format = true;
        }
        "#
    ));
}

#[test]
fn extend_group_field() {
    assert_yaml_snapshot!(check_ok(
        r#"
        message Message {
            extensions 1;
        }

        extend Message {
            repeated group Foo = 1 {
                required int32 bar = 1;
            };
        }
    "#
    ));
}

#[test]
fn extend_field_number_not_in_extensions() {
    assert_eq!(
        check_err(
            r#"
            message Message {
                extensions 2 to 5;
            }

            extend Message {
                optional int32 a = 1;
                repeated int32 b = 6;
            }
            "#
        ),
        vec![
            InvalidExtensionNumber {
                number: 1,
                message_name: "Message".to_owned(),
                help: Some("available extension numbers are 2 to 5".to_owned()),
                span: Some(SourceSpan::from(145..146)),
            },
            InvalidExtensionNumber {
                number: 6,
                message_name: "Message".to_owned(),
                help: Some("available extension numbers are 2 to 5".to_owned()),
                span: Some(SourceSpan::from(183..184)),
            }
        ],
    );
    assert_eq!(
        check_err(
            r#"
            message Message {
                extensions 2 to 5;

                extend Message {
                    optional int32 a = 1;
                    repeated int32 b = 6;
                }
            }
            "#
        ),
        vec![
            InvalidExtensionNumber {
                number: 1,
                message_name: "Message".to_owned(),
                help: Some("available extension numbers are 2 to 5".to_owned()),
                span: Some(SourceSpan::from(139..140)),
            },
            InvalidExtensionNumber {
                number: 6,
                message_name: "Message".to_owned(),
                help: Some("available extension numbers are 2 to 5".to_owned()),
                span: Some(SourceSpan::from(181..182)),
            }
        ],
    );
}

#[test]
#[ignore]
fn extend_duplicate_field_number() {
    // check same extend block
    // different extend block in scope
    // extend block in different scope (e.g. file vs message)
    // defined in imported file
    // defined in file not directly imported
    todo!()
}

#[test]
#[ignore]
fn extend_non_options_type_proto3() {
    todo!()
}

#[test]
fn oneof_group_field() {
    assert_yaml_snapshot!(check_ok(
        r#"
        message Message {
            oneof oneof {
                group Group = 1 {
                    repeated float bar = 1;
                }
            }
        }
        "#
    ))
}

#[test]
fn enum_reserved_range_invalid() {
    assert_eq!(
        check_err(
            r#"enum Enum {
                reserved 1 to -1;
            }"#
        ),
        vec![InvalidRange {
            span: Some(SourceSpan::from(37..44)),
        },],
    );
    assert_yaml_snapshot!(check_ok(
        r#"enum Enum {
            reserved 1 to 1;
        }"#
    ));
}

#[test]
fn enum_reserved_range_overlap() {
    assert_eq!(
        check_err(
            r#"
            syntax = "proto3";

            enum Enum {
                ZERO = 0;

                reserved 3, 3;
            }
            "#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::ReservedRange { start: 3, end: 3 },
            first_span: Some(SourceSpan::from(109..110)),
            second: resolve::NumberKind::ReservedRange { start: 3, end: 3 },
            second_span: Some(SourceSpan::from(112..113)),
        })],
    );
    assert_eq!(
        check_err(
            r#"
            syntax = "proto3";

            enum Enum {
                ZERO = 0;

                reserved 1 to 5, 4;
            }
            "#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::ReservedRange { start: 1, end: 5 },
            first_span: Some(SourceSpan::from(109..115)),
            second: resolve::NumberKind::ReservedRange { start: 4, end: 4 },
            second_span: Some(SourceSpan::from(117..118)),
        })],
    );
    assert_eq!(
        check_err(
            r#"
            syntax = "proto3";

            enum Enum {
                ZERO = 0;

                reserved 3, 2 to max;
            }
            "#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::ReservedRange { start: 3, end: 3 },
            first_span: Some(SourceSpan::from(109..110)),
            second: resolve::NumberKind::ReservedRange {
                start: 2,
                end: 2147483647
            },
            second_span: Some(SourceSpan::from(112..120)),
        })],
    );
}

#[test]
fn enum_reserved_range_overlap_with_value() {
    assert_eq!(
        check_err(
            r#"
            syntax = "proto3";

            enum Enum {
                ZERO = 0;

                reserved -5 to 5;
            }
            "#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::ReservedRange { start: -5, end: 5 },
            first_span: Some(SourceSpan::from(109..116)),
            second: resolve::NumberKind::EnumValue {
                name: "ZERO".to_owned(),
                number: 0
            },
            second_span: Some(SourceSpan::from(80..81)),
        })],
    );
    assert_eq!(
        check_err(
            r#"
            syntax = "proto3";

            enum Enum {
                ZERO = 0;
                FIVE = 5;

                reserved 2 to max;
            }
            "#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::ReservedRange {
                start: 2,
                end: 2147483647
            },
            first_span: Some(SourceSpan::from(135..143)),
            second: resolve::NumberKind::EnumValue {
                name: "FIVE".to_owned(),
                number: 5,
            },
            second_span: Some(SourceSpan::from(106..107)),
        })],
    );
    assert_eq!(
        check_err(
            r#"
            syntax = "proto3";

            enum Enum {
                ZERO = 0;
                FIVE = 5;

                reserved 5;
            }
            "#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::ReservedRange { start: 5, end: 5 },
            first_span: Some(SourceSpan::from(135..136)),
            second: resolve::NumberKind::EnumValue {
                name: "FIVE".to_owned(),
                number: 5,
            },
            second_span: Some(SourceSpan::from(106..107)),
        })],
    );
}

#[test]
fn enum_duplicate_number() {
    assert_eq!(
        check_err(
            r#"
            syntax = "proto3";

            enum Enum {
                ZERO = 0;
                ZERO2 = 0;
            }
            "#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::EnumValue {
                name: "ZERO".to_owned(),
                number: 0
            },
            first_span: Some(SourceSpan::from(80..81)),
            second: resolve::NumberKind::EnumValue {
                name: "ZERO2".to_owned(),
                number: 0
            },
            second_span: Some(SourceSpan::from(107..108)),
        })],
    );
    assert_eq!(
        check_err(
            r#"
            syntax = "proto3";

            message Message {
                enum Enum {
                    option allow_alias = false;

                    ZERO = 0;
                    ZERO2 = 0;
                }
            }
            "#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::EnumValue {
                name: "ZERO".to_owned(),
                number: 0
            },
            first_span: Some(SourceSpan::from(167..168)),
            second: resolve::NumberKind::EnumValue {
                name: "ZERO2".to_owned(),
                number: 0
            },
            second_span: Some(SourceSpan::from(198..199)),
        })],
    );
    assert_yaml_snapshot!(check_ok(
        r#"
        enum Enum {
            option allow_alias = true;

            ZERO = 0;
            ZERO2 = 0;
        }
        "#
    ));
}

#[test]
#[ignore]
fn proto2_enum_in_proto3_message() {
    todo!()
}

#[test]
#[ignore]
fn proto3_enum_default() {
    todo!()
}

#[test]
#[ignore]
fn option_unknown_field() {
    todo!()
}

#[test]
#[ignore]
fn option_unknown_extension() {
    todo!()
}

#[test]
fn option_already_set() {
    assert_eq!(
        check_err(
            r#"
            syntax = 'proto3';

            message Message {
                optional int32 foo = 1 [deprecated = true, deprecated = false];
            }"#
        ),
        vec![OptionAlreadySet {
            name: "deprecated".to_owned(),
            first: Some(SourceSpan::from(103..120)),
            second: Some(SourceSpan::from(122..140))
        }],
    );
}

#[test]
#[ignore]
fn option_ignore() {
    todo!()
}

#[test]
fn option_map_entry_set_explicitly() {
    assert_yaml_snapshot!(check_ok("message Foo { option map_entry = true; }"));
}

#[test]
#[ignore]
fn public_import() {
    todo!()
}
*/

/*
syntax = 'proto2';

import 'google/protobuf/descriptor.proto';

message Foo {
    optional int32 a = 1;
    optional int32 b = 2;
}

extend google.protobuf.FileOptions {
    optional Foo foo = 1001;
}

option (foo).a = 1;

option optimize_for = SPEED;

option (foo).b = 1;
*/

/*

message Foo {
    // hello
    optional group A = 1 {}     ;
}

*/

/*

syntax = 'proto2';

message Foo {
    optional int32 a = 1;

    oneof foo {
        int32 c = 2;
    }

    extensions 3, 6 to max;

    reserved 4 to 5;
    reserved "d", "e";

    extend Foo {
        optional sint32 b = 3;
    }

    message Bar {}

    enum Quz {
        ZERO = 0;
    }

    option deprecated = true;
}

*/

/*
import "google/protobuf/descriptor.proto";
extend google.protobuf.OneofOptions {
  optional int32 my_option = 12345;
}

message Hello {
  oneof something {
    int32 bar = 1;

    option (my_option) = 54321;
  }
}
 */

/*

syntax = 'proto2';

import 'google/protobuf/descriptor.proto';

package exttest;

message Message {
    optional int32 a = 1;
    optional Message b = 3;

    extensions 5 to 6;
}

extend Message {
    optional int32 c = 5;
    optional Message d = 6;
}

extend google.protobuf.FileOptions {
    optional Message foo = 50000;
}

option (exttest.foo).(exttest.d).a = 1;

*/

/*

message Foo {
    optional bytes foo = 20000 [default = "\777"];
}

*/

/*
message Foo {
    optional bytes foo = 20000 [default = "\xFF"];
}
*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a).key = 1;

extend google.protobuf.FileOptions {
    optional Foo.BarEntry a = 1001;
}

message Foo {
    map<int32, string> bar = 1;
    /*optional group A = 1 {

    };*/
}

foo.proto:8:14: map_entry should not be set explicitly. Use map<KeyType, ValueType> instead.


*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a).key = 1;

extend google.protobuf.FileOptions {
    optional Foo.A a = 1001;
}

message Foo {
    optional group A = 1 {
        optional int32 key = 1;
    };
}

*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a) = {
    key: 1, // should fail with numeric keys
};

extend google.protobuf.FileOptions {
    repeated Foo.A a = 1001;
}

message Foo {
    optional group A = 1 {
        optional int32 key = 1;
    };
}

*/

/*

syntax = "proto3";

import "google/protobuf/descriptor.proto";

package demo;

extend google.protobuf.EnumValueOptions {
  optional uint32 len = 50000;
}

enum Foo {
  None = 0 [(len) = 0];
  One = 1 [(len) = 1];
  Two = 2 [(len) = 2];
}

*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a) = {
    /* block */
    # hash
    key: 1.0
    // line
};

extend google.protobuf.FileOptions {
    repeated Foo.A a = 1001;
}

message Foo {
    optional group A = 1 {
        optional float key = 1;
    };
}

*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a) = {
    key:
        "hello"
        "gdfg"
};

extend google.protobuf.FileOptions {
    repeated Foo.A a = 1001;
}

message Foo {
    optional group A = 1 {
        optional string key = 1;
    };
}

*/

/*

syntax =
    "proto"
    "2";

import "google/protobuf/descriptor.proto";

option (a) =
    "hello"
    "gdfg"
;

extend google.protobuf.FileOptions {
    repeated string a = 1001;
}

message Foo {
    optional group A = 1 {
        optional string key = 1;
    };
}

*/

/*

syntax = "proto" "2";

import "google/protobuf/descriptor.proto";

option (a) = {
    key : -inf;
};

extend google.protobuf.FileOptions {
    repeated Foo.A a = 1001;
}

message Foo {
    optional group A = 1 {
        optional float key = 1;
    };
}


*/

/*

syntax = "proto2";

import "google/protobuf/any.proto";
import "google/protobuf/descriptor.proto";

option (a) = {
    [type.googleapis.com/Foo] { foo: "bar" }
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
