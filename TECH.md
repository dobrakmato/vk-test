
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
- positions
- normals
- uvs
- tangents


## Binary Format
Binary format is an optimized format for storing various game files after importing.

#### Header
All files contain header consisting of: 
- magic string 'BF' (u16)
- type number (u8)
- version (u8)
- compressed size (u64)
- uncompressed size (u64)

If the file is not compressed then the `compressed size` should be `0`. 

Right after the header comes the payload (either LZ4 compressed or not). 
Payload data structure depends on the type of file.

#### Types
Following constants are valid type numbers:

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

Types: DXT1, DXT5, RGBA8, RGBA16, RGB8, RGB16, RG16, R16

### Model / Geometry

Each geometry contains multiple lists. Allowed list types:
```
Positions = 0 (float3)
Normals = 1 (float3)
Tangents = 2 (float3)
Colors = 3 (float3)
UV1 = 4 (float2)
UV2 = 5 (float2)
UV3 = 6 (float2)
UV4 = 7 (float2)
Indices = 8 (u32)
```

### Performance

To benchmark:
- `compress(vfs(files))` vs `vfs(compress(file), compress(file))`