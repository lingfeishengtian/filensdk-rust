use uniffi_shared_tokio_runtime_proc::uniffi_async_export;

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    struct MyStruct {
        tokio_runtime: std::sync::Mutex<Option<Runtime>>,
    }

    uniffi::setup_scaffolding!();

    #[uniffi_async_export]
    impl MyStruct {
        pub async fn async_method(&self, value: i32) -> i32 {
            value * 2
        }
    }

    impl MyStruct {
        fn new() -> Self {
            Self {
                tokio_runtime: std::sync::Mutex::new(Some(Runtime::new().unwrap())),
            }
        }
    }

    #[test]
    fn test_async_method() {
        let my_struct = MyStruct::new();
        let result = my_struct.async_method_blocking(5);
        assert_eq!(result, 10);
    }

    // #[test]
    // fn test_blocking_method() {
    //     let my_struct = MyStruct::new();
    //     let result = my_struct.async_method_blocking(5);
    //     assert_eq!(result, 10);
    // }
}
