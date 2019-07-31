/// Enum representing possible types of BF files.
#[repr(u8)]
pub enum Type {
    Image = 0,
    Geometry = 1,
    Audio = 2,
    Material = 3,
    VirtualFileSystem = 4,
    CompiledShader = 5,
    Scene = 6,
}

/// Header of every BF file.
#[repr(C)]
pub struct BfHeader {
    magic: u16,
    kind: Type,
    version: u8,
    uncompressed: u64,
    compressed: u64,
}

pub struct BfFile<'a> {
    header: BfHeader,
    data: &'a [u8],
}

/// Enum representing possible types of images.
#[repr(u8)]
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
#[repr(u8)]
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

// todo: loading & saving of bf files
