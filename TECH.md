
## Conventions

Integers are little-endian.

World space is right-handed.

## Renderer
Rendering pipeline looks like this: 

All visible opaque geometry is rendered to GBuffer to aggregate geometry data.

Then for each light the GBuffer is read and the shading is computed to the HDR buffer.

Then all transparent objects are drawn. 

Then all postprocessing effects are computed.

Lastly the HDR buffer is resolved to LDR buffer using tone-mapping.

### Shading model

We use PBR renderer with following maps available:
- albedo
- roughness
- normal
- ambient occlusion
- emission
- metallic

### Anti-aliasing

MSAA is not supported. FXAA is standard anti-aliasing. TXAA is also supported.


### GBuffer

   Type       |     R     |     G     |     B      |     A     
    -----     |   -----   |   -----   |   -----    |   -----      |
  **D24_S8**  |         Depth  (24 bits)       | | |   Stencil    | 32 bits
  **RGBA8**   |  Albedo.R |  Albedo.G |  Albedo.B  |  Occlusion   | 32 bits
  **RGB10A2** |  Normal.X |  Normal.Y |  Normal.Z  | Light. Model | 32 bits
  **RGBA8**   |  Emissive | Roughness |  Metallic  | Translucency | 32 bits
  **RGBA8**   |  Motion X |  Motion Y | Distortion |              | 32 bits
  
Normals are in world-space.

Valid lighting models are:
- `0` - standard shading

### HDR buffer

   Type       |     R     |     G     |     B     |     A     
    -----     |   -----   |   -----   |   -----   |   -----   |
  **RGB16**   |   HDR.R   |   HDR.G   |   HDR.B   |           | 48 bits


### Vertex buffers

Vertex buffers are not interleaved. Standard shader contains these buffers:
- positions (float3)
- normals (float3)
- uvs (float2)
- tangents (float3)


## Binary Format
Binary format is an optimized format for storing various game files after importing.

#### Header
All files contain header consisting of: 
- magic string 'BF' (u16)
- kind number (u8)
- version (u8)
- *padding* (u32)
- kind dependant data (u64)
- compressed size (u64)
- uncompressed size (u64)

If the file is not compressed then the `compressed size` should be `0`. 

Right after the header comes the payload (either LZ4 compressed or not). 
Payload data structure depends on the type of file.

#### Kinds
Following constants are valid kind numbers:

```
Image = 0
Geometry/Model = 1
Audio = 2
Material = 3
VirtualFileSystem = 4
CompiledShader = 5
Scene = 6 
```

### Virtual File System

VFS files consist of two parts: header(s) and data. They are generally 
uncompressed because their content (individual files) are compressed.

VFS Header consists of:
- number of entries
- entry(ies)
  - name (null terminated utf8 string)
  - length (u32)
  - pointer to start of file (u32)

Right after the header the data part comes.

### Image

Formats: DXT1, DXT3, DXT5, RGB8, RGBA8, (and their srgb variants)

The following values are stored inside the `kind additional data` field of header.
- width (u16)
- height (u16)
- format (u8)
- padding (u8)

The payload contains all mip-maps in the width decreasing order. It is possible to
seek to the n-th mip-map by computing the size of preceding mip-maps using the width,
height and format.


### Model / Geometry

Each geometry contains multiple lists.

The following values are stored inside the `kind additional data` field of header.
- *nothing*

Compressed payload consists of header containing information about the payload which follows the header.

Geometry header:
- global flags (u32)
- num of lists (u32)
- lists
  - list type (u16)
  - list flags (u16)
  - list length (u32)
- payload

Lists are encoded in payload in the same order as they are specified in the header.

Allowed list types:

```
Positions = 0 (float3)
Normals = 1 (float3)
Tangents = 2 (float3)
Colors = 3 (float3)
UV1 = 4 (float2)
UV2 = 5 (float2)
UV3 = 6 (float2)
UV4 = 7 (float2)
Indices(u8) = 8 (u8)
Indices(u16) = 9 (u16)
Indices(u32) = 10 (u32)
```



### Performance

To benchmark:
- `compress(vfs(files))` vs `vfs(compress(file), compress(file))`
