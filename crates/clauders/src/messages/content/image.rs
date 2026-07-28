//! Image input block (vision) for the request content-block union.
//!
//! Exists as its own file so the vision block and its source variants are
//! scoped apart from the other request blocks. Referenced only by
//! [`super::ContentBlockParam::Image`].

/// Media type of a base64-encoded image, a closed four-value set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ImageMediaType {
    /// `image/jpeg`.
    #[serde(rename = "image/jpeg")]
    Jpeg,
    /// `image/png`.
    #[serde(rename = "image/png")]
    Png,
    /// `image/gif`.
    #[serde(rename = "image/gif")]
    Gif,
    /// `image/webp`.
    #[serde(rename = "image/webp")]
    Webp,
}

/// Source of an image: inline base64 bytes or a fetchable URL.
///
/// The Files API `file` source is beta and intentionally omitted.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    /// Inline base64-encoded image bytes with their media type.
    Base64 {
        /// The image's media type.
        media_type: ImageMediaType,
        /// The base64-encoded image bytes.
        data: String,
    },
    /// A URL the API fetches the image from.
    Url {
        /// The image URL.
        url: String,
    },
}

/// An image content block a caller sends for vision input.
///
/// # Examples
///
/// ```
/// use clauders::messages::content::image::{ImageBlock, ImageSource, ImageMediaType};
/// let block = ImageBlock {
///     source: ImageSource::Base64 { media_type: ImageMediaType::Png, data: "iVBOR".into() },
///     cache_control: None,
/// };
/// let j = serde_json::to_value(&block).unwrap();
/// assert_eq!(j["source"]["type"], "base64");
/// assert_eq!(j["source"]["media_type"], "image/png");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ImageBlock {
    /// Where the image bytes come from.
    pub source: ImageSource,
    /// Optional cache breakpoint for this block.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cache_control: Option<crate::types::CacheControl>,
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;

    #[test]
    fn base64_image_serializes_with_media_type() {
        let block = ImageBlock {
            source: ImageSource::Base64 {
                media_type: ImageMediaType::Jpeg,
                data: "AAAA".into(),
            },
            cache_control: None,
        };
        let j = serde_json::to_value(&block).unwrap();
        assert_eq!(j["source"]["type"], "base64");
        assert_eq!(j["source"]["media_type"], "image/jpeg");
        assert_eq!(j["source"]["data"], "AAAA");
    }

    #[test]
    fn url_image_round_trips() {
        let original = ImageBlock {
            source: ImageSource::Url {
                url: "https://example.com/x.png".into(),
            },
            cache_control: None,
        };
        let j = serde_json::to_string(&original).unwrap();
        let back: ImageBlock = serde_json::from_str(&j).unwrap();
        assert_eq!(back, original);
    }

    #[test]
    fn all_four_media_types_round_trip() {
        for mt in [
            ImageMediaType::Jpeg,
            ImageMediaType::Png,
            ImageMediaType::Gif,
            ImageMediaType::Webp,
        ] {
            let j = serde_json::to_string(&mt).unwrap();
            let back: ImageMediaType = serde_json::from_str(&j).unwrap();
            assert_eq!(back, mt);
        }
    }
}
