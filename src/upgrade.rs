pub fn next_version(current: &str) -> String {
    let Some(number) = current.strip_prefix('v') else {
        return "v1".to_string();
    };
    match number.parse::<u32>() {
        Ok(value) => format!("v{}", value + 1),
        Err(_) => "v1".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn increments_simple_v_versions() {
        assert_eq!(next_version("v0"), "v1");
        assert_eq!(next_version("v9"), "v10");
        assert_eq!(next_version("custom"), "v1");
    }
}
