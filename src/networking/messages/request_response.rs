#[macro_export]
macro_rules! request_response {
    ($msg:ty) => {
        request_response!($msg, request_identifier);
    };
    ($msg:ty, $a:ident) => {
        impl networking::messages::message_interface::RequestMessage for $msg {
            fn set_request_identifier(&mut self, request_identifier: u32) {
                self.$a = request_identifier;
            }
        }
        impl networking::messages::message_interface::ResponseMessage for $msg {
            fn get_request_identifier(&self) -> u32 {
                self.$a
            }
        }
    };
}
