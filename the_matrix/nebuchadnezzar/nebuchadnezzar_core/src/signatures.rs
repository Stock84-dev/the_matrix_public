use hex;
use reqwest::{Method, Url};
use ring::hmac;
use ring::hmac::Key;

pub fn hmac_sha256(key: &Key, method: Method, expires: i64, url: &Url, body: &str) -> String {
    let sign_message = match url.query() {
        Some(query) => format!(
            "{}{}?{}{}{}",
            method.as_str(),
            url.path(),
            query,
            expires,
            body
        ),
        None => format!("{}{}{}{}", method.as_str(), url.path(), expires, body),
    };
    let signature = hex::encode(hmac::sign(&key, sign_message.as_bytes()));
    signature
}

// #[cfg(test)]
// mod test {
//     use crate::client::{RequestHeaders, Requestable};
//     use crate::reqwest::{Client, RequestBuilder};
//     use crate::Credentials;
//     use mouse::error::Result;
//     use reqwest::{Method, Url};
//     use crate::signatures::hmac_sha256;
//
//     #[test]
//     fn test_signature_get() -> Result<()> {
//         let sig = hmac_sha256()
//         let tr = MockClient::with_credentials(
//         );
//         let (_, sig) = tr.signature(
//             Method::GET,
//             1518064236,
//             &Url::parse("http://a.com/api/v1/instrument")?,
//             "",
//         )?;
//         assert_eq!(
//             sig,
//             "c7682d435d0cfe87c16098df34ef2eb5a549d4c5a3c2b1f0f77b8af73423bf00"
//         );
//         Ok(())
//     }
//
//     #[test]
//     fn test_signature_get_param() -> Result<()> {
//         let tr = MockClient::with_credentials(
//         );
//         let (_, sig) = tr.signature(
//             Method::GET,
//             1518064237,
//             &Url::parse_with_params(
//                 "http://a.com/api/v1/instrument",
//                 &[("filter", r#"{"symbol": "XBTM15"}"#)],
//             )?,
//             "",
//         )?;
//         assert_eq!(
//             sig,
//             "e2f422547eecb5b3cb29ade2127e21b858b235b386bfa45e1c1756eb3383919f"
//         );
//         Ok(())
//     }
//
//     #[test]
//     fn test_signature_post() -> Result<()> {
//         let tr = MockClient::with_credentials(
//         );
//         let (_, sig) = tr.signature(
//             Method::POST,
//             1518064238,
//             &Url::parse("http://a.com/api/v1/order")?,
//             r#"{"symbol":"XBTM15","price":219.0,"clOrdID":"mm_bitmex_1a/oemUeQ4CAJZgP3fjHsA","orderQty":98}"#,
//         )?;
//         assert_eq!(
//             sig,
//             "1749cd2ccae4aa49048ae09f0b95110cee706e0944e6a14ad0b3a8cb45bd336b"
//         );
//         Ok(())
//     }
// }
