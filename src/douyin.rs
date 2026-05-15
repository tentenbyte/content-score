use anyhow::Result;
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum DouyinCommand {
    Doctor,
    Login,
    Fetch {
        prediction_id: String,
        input: String,
        #[arg(long)]
        no_import: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        replace: bool,
    },
}

pub fn handle(command: DouyinCommand) -> Result<()> {
    match command {
        DouyinCommand::Doctor => {
            anyhow::bail!("adapter checks are not implemented yet");
        }
        DouyinCommand::Login => {
            anyhow::bail!("adapter login is not implemented yet");
        }
        DouyinCommand::Fetch {
            prediction_id,
            input,
            no_import,
            dry_run,
            replace,
        } => {
            let resolved = resolve_aweme_id(&input)?;
            println!("douyin fetch stub for prediction {prediction_id}");
            println!("input: {resolved}");
            if no_import {
                println!("no-import: true");
            }
            if dry_run {
                println!("dry-run: true");
            }
            if replace {
                println!("replace: true");
            }
            anyhow::bail!("adapter fetch is not implemented yet");
        }
    }
}

pub fn resolve_aweme_id(input: &str) -> Result<String> {
    let input = input.trim();
    if !input.is_empty() && input.chars().all(|c| c.is_ascii_digit()) {
        return Ok(input.to_string());
    }

    if let Some((host, path)) = parse_http_url(input) {
        if host == "v.douyin.com" {
            return Ok(input.to_string());
        }

        if matches!(host.as_str(), "douyin.com" | "www.douyin.com") {
            if let Some(aweme_id) = video_id_from_path(path) {
                return Ok(aweme_id.to_string());
            }
        }
    }

    anyhow::bail!(
        "unsupported Douyin input: expected raw aweme id, douyin.com/video/<id>, or v.douyin.com short link"
    )
}

fn parse_http_url(input: &str) -> Option<(String, &str)> {
    let rest = input
        .strip_prefix("https://")
        .or_else(|| input.strip_prefix("http://"))?;
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let host = host
        .split_once(':')
        .map_or(host, |(host_without_port, _)| host_without_port)
        .to_ascii_lowercase();

    Some((host, path))
}

fn video_id_from_path(path: &str) -> Option<&str> {
    let mut segments = path.split('/');
    while let Some(segment) = segments.next() {
        if segment == "video" {
            let aweme_id = segments
                .next()?
                .split(['?', '#'])
                .next()
                .unwrap_or_default();
            if !aweme_id.is_empty() && aweme_id.chars().all(|c| c.is_ascii_digit()) {
                return Some(aweme_id);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_raw_and_long_douyin_inputs() {
        assert_eq!(
            resolve_aweme_id("7333333333333333333").unwrap(),
            "7333333333333333333"
        );
        assert_eq!(
            resolve_aweme_id("https://www.douyin.com/video/7333333333333333333").unwrap(),
            "7333333333333333333"
        );
        assert_eq!(
            resolve_aweme_id("https://douyin.com/video/7333333333333333333").unwrap(),
            "7333333333333333333"
        );
    }

    #[test]
    fn accepts_v_douyin_short_link_for_adapter_resolution() {
        assert_eq!(
            resolve_aweme_id("https://v.douyin.com/iF8abc1/").unwrap(),
            "https://v.douyin.com/iF8abc1/"
        );
    }

    #[test]
    fn rejects_invalid_douyin_input() {
        let error = resolve_aweme_id("https://example.com/video/7333333333333333333").unwrap_err();

        assert!(error.to_string().contains("Douyin"));
    }

    #[test]
    fn stubs_return_not_implemented_errors() {
        let doctor_error = handle(DouyinCommand::Doctor).unwrap_err();
        assert!(doctor_error
            .to_string()
            .contains("adapter checks are not implemented yet"));

        let login_error = handle(DouyinCommand::Login).unwrap_err();
        assert!(login_error
            .to_string()
            .contains("adapter login is not implemented yet"));
    }
}
