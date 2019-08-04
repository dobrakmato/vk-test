use zerocopy::LayoutVerified;
use byteorder::{LittleEndian, ByteOrder};

/// Enum representing possible types of BF files.
pub enum Type {
    Image = 0,
    Geometry = 1,
    Audio = 2,
    Material = 3,
    VirtualFileSystem = 4,
    CompiledShader = 5,
    Scene = 6,
    MaxValue = 7,
}

/// Header of every BF file.
#[repr(C)]
#[derive(FromBytes, AsBytes, Eq, PartialEq, Hash, Debug)]
pub struct BfHeader {
    magic: u16,
    kind: u8,
    version: u8,
    reserved: u32,
    pub uncompressed: u64,
    pub compressed: u64,
}

/* Constant representing the two byte magic sequence 'BF' */
const BF_MAGIC: u16 = 17986;
const BF_MAX_SUPPORTED_VERSION: u8 = 1;

impl BfHeader {
    pub fn new(kind: Type, version: u8, uncompressed: u64, compressed: u64) -> Self {
        BfHeader {
            magic: BF_MAGIC,
            kind: kind as u8,
            reserved: 0,
            version,
            uncompressed,
            compressed,
        }
    }
}

/// Structure for holding loaded BfFile using zero-copy loading mechanism.
#[derive(Debug)]
pub struct BfFile<'a> {
    header: LayoutVerified<&'a [u8], BfHeader>,
    data: &'a [u8],
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
    if bytes[2] > Type::MaxValue as u8 { return Err(Error::InvalidKindValue); }
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
    use crate::bf::{BfHeader, Type, load_bf_from_bytes, Error, BF_MAX_SUPPORTED_VERSION, BF_MAGIC};

    #[test]
    fn test_load_bf_from_bytes() {
        let header = BfHeader {
            magic: BF_MAGIC,
            kind: Type::CompiledShader as u8,
            version: 1,
            reserved: 0,
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
        assert_eq!(file.data, data);
    }

    #[test]
    fn test_invalid_header_variants() {
        assert_matches!(load_bf_from_bytes(&[0, 0, 0]), Err(Error::InvalidFileSignature));
        assert_matches!(load_bf_from_bytes(&[66, 70, 255]), Err(Error::InvalidKindValue));
        assert_matches!(load_bf_from_bytes(&[66, 70, 1, BF_MAX_SUPPORTED_VERSION + 1]), Err(Error::VersionTooHigh));
        assert_matches!(load_bf_from_bytes(&[66, 70, 1, 1, 0, 1]), Err(Error::NotEnoughDataOrUnaligned));
    }
}