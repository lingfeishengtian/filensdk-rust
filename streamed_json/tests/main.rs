#[cfg(test)]
mod tests {
    use streamed_json::{ReaderDeserializableExt, StreamedJsonDeserializable, iter_json_array};
    use streamed_json_proc_macro::streamed_json;

    #[derive(Debug)]
    #[derive(PartialEq, Clone)]
    #[streamed_json(lowerCamelCase)]
    enum Test {
        Field1(String),
        FieldDeserializableStruct(TestDeserializable),
        Field2(i32),
    }

    #[derive(serde::Deserialize)]
    #[derive(PartialEq, Clone, Debug)]
    struct TestDeserializable {
        a: String,
        b: i32,
    }

    fn convert_str_to_vec_test(test_str: &str) -> Vec<Test> {
        let reader = std::io::Cursor::new(test_str);
        let iter = iter_json_array(reader);

        let collected_with_err: Vec<std::io::Result<Test>> = iter.collect();
        let len = collected_with_err.len();
        let collected: Vec<Test> = collected_with_err
            .into_iter()
            .map(|result| result.unwrap())
            .collect();
        assert_eq!(collected.len(), len);
        collected
    }

    #[test]
    fn test_empty_array() {
        let str = r#"{
            "field1": [],
            "field2": []
        }"#;
        let collected = convert_str_to_vec_test(str);
        assert_eq!(collected.len(), 0);
    }

    #[test]
    fn test_single_element_array() {
        let str = r#"{
            "field1": ["value1"],
            "field2": [42]
        }"#;
        let collected = convert_str_to_vec_test(str);
        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0], Test::Field1("value1".to_string()));
        assert_eq!(collected[1], Test::Field2(42));
    }

    #[test]
    fn test_multiple_element_array() {
        let str = r#"{
            "field1": ["value1", "value2"],
            "field2": [42, 43]
        }"#;
        let collected = convert_str_to_vec_test(str);
        assert_eq!(collected.len(), 4);
        assert_eq!(collected[0], Test::Field1("value1".to_string()));
        assert_eq!(collected[1], Test::Field1("value2".to_string()));
        assert_eq!(collected[2], Test::Field2(42));
        assert_eq!(collected[3], Test::Field2(43));
    }

    #[test]
    fn test_second_array_empty() {
        let str = r#"{
            "field1": ["value1", "value2"],
            "field2": []
        }"#;
        let collected = convert_str_to_vec_test(str);
        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0], Test::Field1("value1".to_string()));
        assert_eq!(collected[1], Test::Field1("value2".to_string()));
    }

    #[test]
    fn test_first_array_empty() {
        let str = r#"{
            "field1": [],
            "field2": [42, 43]
        }"#;
        let collected = convert_str_to_vec_test(str);
        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0], Test::Field2(42));
        assert_eq!(collected[1], Test::Field2(43));
    }

    #[test]
    fn weird_characters() {
        let str = r#"{
            "field1": [          "valu.,;'1@#%^$*)```     ~~~~e1"       ,       "va---lue2==============    "   ]       ,


            
            "field2": [ 42, 43    ]
        }"#;

        let collected = convert_str_to_vec_test(str);
        assert_eq!(collected.len(), 4);
        assert_eq!(collected[0], Test::Field1("valu.,;'1@#%^$*)```     ~~~~e1".to_string()));
        assert_eq!(collected[1], Test::Field1("va---lue2==============    ".to_string()));
        assert_eq!(collected[2], Test::Field2(42));
        assert_eq!(collected[3], Test::Field2(43));
    }

    #[test]
    fn test_deserializable_struct() {
        let str = r#"{
            "fieldDeserializableStruct": [
                {"a": "value1", "b": 42},
                {"a": "value2", "b": 43}
            ]
        }"#;

        let collected = convert_str_to_vec_test(str);
        assert_eq!(collected.len(), 2);
        assert_eq!(
            collected[0],
            Test::FieldDeserializableStruct(TestDeserializable {
                a: "value1".to_string(),
                b: 42
            })
        );
        assert_eq!(
            collected[1],
            Test::FieldDeserializableStruct(TestDeserializable {
                a: "value2".to_string(),
                b: 43
            })
        );
    }


    #[test]
    fn test_deserializable_struct_before_other_struct() {
        let str = r#"{
            "fieldDeserializableStruct": [
                {"a": "value1", "b": 42},
                {"a": "value2", "b": 43}
            ],
            "field1": ["value1", "value2"],
        }"#;

        let collected = convert_str_to_vec_test(str);
        assert_eq!(collected.len(), 4);
        assert_eq!(
            collected[0],
            Test::FieldDeserializableStruct(TestDeserializable {
                a: "value1".to_string(),
                b: 42
            })
        );
        assert_eq!(
            collected[1],
            Test::FieldDeserializableStruct(TestDeserializable {
                a: "value2".to_string(),
                b: 43
            })
        );
    }
}
