pub fn format_url_string(host: [u8; 4], port: u16) -> String {
    let host_string = host
        .iter()
        .map(|int| int.to_string())
        .collect::<Vec<String>>()
        .join(".");
    format!("{}{}{}", host_string, ":", port.to_string(),)
}
