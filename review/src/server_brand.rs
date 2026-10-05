//! Which public server a moto was recorded on, from the stored server name.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServerBrand {
    MxbRanked,
    Cbr,
}

pub fn server_brand(server_name: &str) -> Option<ServerBrand> {
    let name = server_name.to_ascii_lowercase();
    if name.contains("mxb-ranked") {
        return Some(ServerBrand::MxbRanked);
    }
    if name.contains("cbrservers") || server_name_has_cbr_token(server_name) {
        return Some(ServerBrand::Cbr);
    }
    None
}

fn server_name_has_cbr_token(server_name: &str) -> bool {
    server_name
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|part| part.eq_ignore_ascii_case("cbr"))
}

#[cfg(test)]
mod tests {
    use super::{server_brand, ServerBrand};

    #[test]
    fn server_brand_matches_ranked_and_cbr() {
        assert_eq!(
            server_brand("MXB-Ranked.com | MX Freebies | 250 | S3- | EU | #105573"),
            Some(ServerBrand::MxbRanked)
        );
        assert_eq!(server_brand("mxb-ranked.com"), Some(ServerBrand::MxbRanked));
        assert_eq!(
            server_brand("1 | OPEN OEM | Stock Track Rotation 1 | CBRSERVERS.COM"),
            Some(ServerBrand::Cbr)
        );
        assert_eq!(server_brand("CBR"), Some(ServerBrand::Cbr));
        assert_eq!(server_brand("Some casual server"), None);
        assert_eq!(server_brand(""), None);
    }
}
