use zerocopy::LayoutVerified;
use byteorder::{LittleEndian, ByteOrder};
use crate::bf::ColorSpace::{Linear, Srgb};
use crate::bf::BfImageFormat::{Dxt1, Dxt3, Dxt5, Rgb8, Rgba8, Srgb8, Srgb8A8, SrgbDxt5, SrgbDxt3, SrgbDxt1};
use std::convert::TryFrom;
use crate::bf::Kind::{Image, Geometry, Audio, Material, VirtualFileSystem, CompiledShader, Scene};

/// Enum representing possible types of BF files.
#[derive(Debug)]
#[repr(u8)]
pub enum Kind {
    Image = 0,
    Geometry = 1,
    Audio = 2,
    Material = 3,
    VirtualFileSystem = 4,
    CompiledShader = 5,
    Scene = 6,
    MaxValue = 7,
}

// todo: rather derive than manually implement
impl TryFrom<u8> for Kind {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Image),
            1 => Ok(Geometry),
            2 => Ok(Audio),
            3 => Ok(Material),
            4 => Ok(VirtualFileSystem),
            5 => Ok(CompiledShader),
            6 => Ok(Scene),
            _ => Err(()),
        }
    }
}

/// Header of every BF file.
#[repr(C)]
#[derive(FromBytes, AsBytes, Eq, PartialEq, Hash, Debug)]
pub struct BfHeader {
    pub magic: u16,
    pub kind: u8,
    pub version: u8,
    pub reserved: u32,
    pub additional: u64,
    pub uncompressed: u64,
    pub compressed: u64,
}

/// Struct for additional data of Image kind.
#[repr(C)]
#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone)]
pub struct BfImageAdditional {
    width: u16,
    height: u16,
    format: u8,
    padding1: u8,
    padding2: u16,
}

impl BfImageAdditional {
    pub fn new(width: u16, height: u16, format: u8) -> Self {
        BfImageAdditional {
            width,
            height,
            format,
            padding1: 0,
            padding2: 0,
        }
    }

    pub fn into_u64(self) -> u64 {
        return unsafe { std::mem::transmute(self) };
    }

    pub fn from_u64(data: u64) -> Self {
        return unsafe { std::mem::transmute(data) };
    }
}

pub enum ColorSpace {
    Linear,
    Srgb,
}

/// Supported image formats in Image kind of BF files.
#[repr(u8)]
pub enum BfImageFormat {
    // linear variants
    Dxt1 = 0,
    Dxt3 = 1,
    Dxt5 = 2,
    Rgb8 = 3,
    Rgba8 = 4,
    // srgb variants
    SrgbDxt1 = 5,
    SrgbDxt3 = 6,
    SrgbDxt5 = 7,
    Srgb8 = 8,
    Srgb8A8 = 9,
}

impl BfImageFormat {
    pub fn channels(&self) -> usize {
        match self {
            BfImageFormat::Dxt1 => 3,
            BfImageFormat::Dxt3 => 4,
            BfImageFormat::Dxt5 => 4,
            BfImageFormat::Rgb8 => 3,
            BfImageFormat::Rgba8 => 4,
            BfImageFormat::SrgbDxt1 => 3,
            BfImageFormat::SrgbDxt3 => 4,
            BfImageFormat::SrgbDxt5 => 4,
            BfImageFormat::Srgb8 => 3,
            BfImageFormat::Srgb8A8 => 4,
        }
    }

    pub fn color_space(&self) -> ColorSpace {
        match self {
            BfImageFormat::SrgbDxt1 => Srgb,
            BfImageFormat::SrgbDxt3 => Srgb,
            BfImageFormat::SrgbDxt5 => Srgb,
            BfImageFormat::Srgb8 => Srgb,
            BfImageFormat::Srgb8A8 => Srgb,
            _ => Linear
        }
    }

    pub fn from_string(s: &str) -> Option<BfImageFormat> {
        match s {
            "dxt1" => Some(Dxt1),
            "dxt3" => Some(Dxt3),
            "dxt5" => Some(Dxt5),
            "rgb" => Some(Rgb8),
            "rgba" => Some(Rgba8),
            "srgb_dxt1" => Some(SrgbDxt1),
            "srgb_dxt3" => Some(SrgbDxt3),
            "srgb_dxt5" => Some(SrgbDxt5),
            "srgb" => Some(Srgb8),
            "srgb_a" => Some(Srgb8A8),
            _ => None
        }
    }
}

/* Constant representing the two byte magic sequence 'BF' */
const BF_MAGIC: u16 = 17986;
const BF_MAX_SUPPORTED_VERSION: u8 = 1;

impl BfHeader {
    pub fn new(kind: Kind, version: u8, additional: u64, uncompressed: u64, compressed: u64) -> Self {
        BfHeader {
            magic: BF_MAGIC,
            kind: kind as u8,
            version,
            reserved: 0,
            additional,
            uncompressed,
            compressed,
        }
    }
}

/// Structure for holding loaded BfFile using zero-copy loading mechanism.
#[derive(Debug)]
pub struct BfFile<'a> {
    pub header: LayoutVerified<&'a [u8], BfHeader>,
    pub data: &'a [u8],
}

/// Enum representing possible types of images.
pub enum ImageType {
    DXT1,
    DXT5,
    RGBA8,
    RGBA16,
    RGB8,
    RGB16,
    RG16,
    R16,
}

/// Enum representing possible types geometry lists.
pub enum GeometryListType {
    Positions = 0,
    Normals = 1,
    Tangents = 2,
    Colors = 3,
    UV1 = 4,
    UV2 = 5,
    UV3 = 6,
    UV4 = 7,
    Indices = 8,
}

pub struct GeometryList<'a> {
    kind: GeometryListType,
    length: usize,
    data: &'a [u8],
}

#[derive(Debug)]
pub enum Error {
    NotEnoughDataOrUnaligned,
    InvalidFileSignature,
    VersionTooHigh,
    InvalidKindValue,
}

/// Loads and deserializes byte array to BfFile using zero-copy mechanism. If
/// the specified byte sequence is invalid Error is returned.
pub fn load_bf_from_bytes(bytes: &[u8]) -> Result<BfFile, Error> {

    // verify magic, version and kind values
    if LittleEndian::read_u16(bytes) != BF_MAGIC { return Err(Error::InvalidFileSignature); }
    if bytes[2] > Kind::MaxValue as u8 { return Err(Error::InvalidKindValue); }
    if bytes[3] > 1 { return Err(Error::VersionTooHigh); }

    // transmute the slice
    match LayoutVerified::new_from_prefix(bytes) {
        None => Err(Error::NotEnoughDataOrUnaligned),
        Some((header, data)) => Ok(BfFile { header, data })
    }
}


#[cfg(test)]
mod tests {
    use matches::assert_matches;
    use zerocopy::AsBytes;
    use crate::bf::{BfHeader, Kind, load_bf_from_bytes, Error, BF_MAX_SUPPORTED_VERSION, BF_MAGIC, BfImageAdditional};

    #[test]
    fn test_load_bf_from_bytes() {
        let header = BfHeader {
            magic: BF_MAGIC,
            kind: Kind::CompiledShader as u8,
            version: 1,
            reserved: 0,
            additional: 66,
            uncompressed: 1024,
            compressed: 1023,
        };
        let data = &[1, 2, 3, 4, 1, 2, 3, 4];

        let mut bytes = Vec::new();
        bytes.extend(header.as_bytes());
        bytes.extend(data.as_bytes());

        // load from bytes
        let result = load_bf_from_bytes(&bytes);

        assert!(result.is_ok());

        let file = result.ok().unwrap();

        assert_eq!(file.header.magic, header.magic);
        assert_eq!(file.header.kind, header.kind);
        assert_eq!(file.header.version, header.version);
        assert_eq!(file.header.reserved, header.reserved);
        assert_eq!(file.header.uncompressed, header.uncompressed);
        assert_eq!(file.header.compressed, header.compressed);
        assert_eq!(file.header.additional, header.additional);
        assert_eq!(file.data, data);
    }

    #[test]
    fn test_invalid_header_variants() {
        assert_matches!(load_bf_from_bytes(&[0, 0, 0]), Err(Error::InvalidFileSignature));
        assert_matches!(load_bf_from_bytes(&[66, 70, 255]), Err(Error::InvalidKindValue));
        assert_matches!(load_bf_from_bytes(&[66, 70, 1, BF_MAX_SUPPORTED_VERSION + 1]), Err(Error::VersionTooHigh));
        assert_matches!(load_bf_from_bytes(&[66, 70, 1, 1, 0, 1]), Err(Error::NotEnoughDataOrUnaligned));
    }

    #[test]
    fn bf_image_additional_data() {
        let a = BfImageAdditional {
            width: 169,
            height: 444,
            format: 4,
            padding1: 0,
            padding2: 0,
        };

        let b = a.clone();
        let a_u64 = b.into_u64();

        assert_eq!(a, BfImageAdditional::from_u64(a_u64));
    }
}