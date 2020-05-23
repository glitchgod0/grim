use crate::grim::io::compression::*;
use crate::grim::io::stream::Stream;
use std::fmt::{Display, Formatter};
use std::path::Path;
use thiserror::Error;

#[derive(Copy, Clone, Debug)]
pub enum BlockStructure {
    TypeA(u32), // Block structure, no compression
    TypeB(u32), // Block structure, zlib compression
    TypeC(u32), // Block structure, gzip compression
    TypeD(u32), // Block structure, zlib compression (with inflate sizes block prefixed)
    Uncompressed,
}

#[derive(Debug)]
pub struct MiloArchive {
    structure: BlockStructure,
    data: Vec<u8>
}

#[derive(Debug, Error)]
pub enum MiloBlockStructureError {
    #[error("Unsupported compression with magic of 0x{magic:X}")]
    UnsupportedCompression {
        magic: u32
    }
}

impl MiloArchive {
    pub fn from_stream(stream: &mut Box<dyn Stream>) -> Result<MiloArchive, Box<dyn std::error::Error>> {
        let magic = MiloArchive::read_magic_and_offset(stream)?;

        let reader = stream.as_mut();

        let block_count = reader.read_int32()?;
        let max_inflate_size = reader.read_int32()?;

        let mut block_sizes: Vec<i32> = vec![0; block_count as usize];

        for (_, size) in block_sizes
            .iter_mut()
            .enumerate() {
            *size = reader.read_int32()?;
        }

        let offset = match magic {
            BlockStructure::TypeA(offset) => offset,
            BlockStructure::TypeB(offset) => offset,
            BlockStructure::TypeC(offset) => offset,
            BlockStructure::TypeD(offset) => offset,
            BlockStructure::Uncompressed => 0,
        };

        // Advances to first block
        reader.seek(offset as u64)?;

        let mut uncompressed: Vec<u8> = Vec::new();

        for block_size in block_sizes.iter() {
            let bytes = reader.read_bytes(*block_size as usize)?;

            let mut data = inflate_zlib_block(&bytes, max_inflate_size as usize)?;
            uncompressed.append(&mut data);
        }

        Ok(MiloArchive {
            structure: magic,
            data: uncompressed
        })
    }

    fn read_magic_and_offset(stream: &mut Box<dyn Stream>) -> Result<BlockStructure, Box<dyn std::error::Error>> {
        let reader = stream.as_mut();

        // TODO: Read as u32
        let magic = reader.read_int32()? as u32;
        let block_offset = reader.read_int32()? as u32;

        match magic {
            0xCABEDEAF => Ok(BlockStructure::TypeA(block_offset)),
            0xCBBEDEAF => Ok(BlockStructure::TypeB(block_offset)),
            0xCCBEDEAF => Ok(BlockStructure::TypeC(block_offset)),
            0xCDBEDEAF => Ok(BlockStructure::TypeD(block_offset)),
            _ => Err(Box::new(MiloBlockStructureError::UnsupportedCompression { magic }))
        }
    }
}
