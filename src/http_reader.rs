
#[cfg(test)]
mod test {
    use crate::test_utils::*;
    use crate::test_server::*;

    #[test]
    fn test_server() {
        init_logging();

        let _server = create_test_server("http-roots/test1");

    }
}