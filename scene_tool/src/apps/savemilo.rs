use crate::apps::{GameOptions, SubApp};
use clap::{Clap};
use std::cmp::Ordering;
use std::error::Error;
use std::{arch, fs};
use std::path::{Path, PathBuf};
use thiserror::Error;

use grim::{Platform, SystemInfo};
use grim::io::*;
use grim::scene::{Object, ObjectDir, ObjectReadWrite, PackedObject, Tex};
use grim::texture::{write_rgba_to_file};


#[derive(Clap, Debug)]
pub struct SaveMiloApp {
    #[clap(about = "Path to input milo scene", required = true)]
    pub in_milo_path: String,
    #[clap(about = "Path to output milo scene", required = true)]
    pub out_milo_path: String,
    #[clap(short = 'm', long, about = "Milo archive version (10, 24, 25)")]
    pub milo_version: Option<u32>,
    #[clap(short = 'b' , long, about = "Use big endian serialization")]
    pub big_endian: Option<bool>,
    #[clap(short = 'u' , long, about = "Leave output milo archive uncompressed")]
    pub uncompressed: bool,
}

impl SubApp for SaveMiloApp {
    fn process(&mut self) -> Result<(), Box<dyn Error>> {
        let in_milo_path = Path::new(&self.in_milo_path);
        let out_milo_path = Path::new(&self.out_milo_path);

        if let Some(file_name) = in_milo_path.file_name() {
            let file_name = file_name.to_str().unwrap();
            println!("Opening {}", file_name);
        }

        // Open milo file
        let mut stream: Box<dyn Stream> = Box::new(FileStream::from_path_as_read_open(in_milo_path)?);
        let milo = MiloArchive::from_stream(&mut stream)?;

        // Guess platform info
        let in_sys_info = SystemInfo::guess_system_info(&milo, in_milo_path);
        let mut out_sys_info = SystemInfo {
            platform: Platform::guess_platform(out_milo_path),
            ..in_sys_info
        };

        if in_sys_info.platform.ne(&out_sys_info.platform) && out_sys_info.platform.eq(&Platform::Wii) {
            todo!("Converting milo to wii is not currently supported!");
        }

        if out_sys_info.platform.eq(&Platform::PS3) {
            // Force big endian if ps3
            out_sys_info.endian = IOEndian::Big;
        }

        // Set milo version if provided
        if let Some(mv) = self.milo_version {
            out_sys_info.version = mv;
            println!("Using milo version: {}", mv);
        }

        // Set endian if provided
        if let Some(end) = self.big_endian {
            out_sys_info.endian = match end {
                true => IOEndian::Big,
                _ => IOEndian::Little
            };
            println!("Using endian: {:?}", out_sys_info.endian);

            if in_sys_info.endian.ne(&out_sys_info.endian) {
                println!("WARN: Output endian doesn't match input");
            }
        }

        // Unpack milo
        let mut obj_dir = milo.unpack_directory(&in_sys_info)?;
        obj_dir.unpack_entries(&in_sys_info).unwrap();

        if in_sys_info.platform.ne(&out_sys_info.platform) {
            println!("Converting platform from {:?} to {:?}", in_sys_info.platform, out_sys_info.platform);

            convert_textures(&mut obj_dir, &in_sys_info, &out_sys_info);
        }

        if in_sys_info.version.ne(&out_sys_info.version) {
            println!("Converting milo version from {:?} to {:?}", in_sys_info.version, out_sys_info.version);
        }

        // Write to new milo archive
        let block_type = self.uncompressed.then(|| BlockType::TypeA);
        let archive = MiloArchive::from_object_dir(&obj_dir, &out_sys_info, block_type)?;

        let mut stream = FileStream::from_path_as_read_write_create(out_milo_path)?;
        archive.write_to_stream(&mut stream)?;

        if let Some(file_name) = out_milo_path.file_name() {
            let file_name = file_name.to_str().unwrap();
            println!("Successfully wrote {}", file_name);
        }

        Ok(())
    }
}

fn convert_textures(milo_dir: &mut ObjectDir, in_sys_info: &SystemInfo, out_sys_info: &SystemInfo) {
    for obj in milo_dir.get_entries_mut() {
        if let Object::Tex(tex) = obj {
            transcode_texture(tex, in_sys_info, out_sys_info);
        }
    }
}

fn transcode_texture(tex: &mut Tex, in_sys_info: &SystemInfo, out_sys_info: &SystemInfo) {
    if let Some(bitmap) = &mut tex.bitmap {
        match (bitmap.encoding, &in_sys_info.platform, &out_sys_info.platform) {
            (32, &Platform::X360, _) => {},
            (_, &Platform::PS3, &Platform::X360) | (_, &Platform::X360, &Platform::PS3) => {
                // Just swap bytes to save time
                if bitmap.raw_data.len() > 32 {
                    swap_image_bytes(&mut bitmap.raw_data[32..]);
                    println!("Swapped bytes for \"{}\"", tex.name.as_str());
                }
                return;
            },
            _ => {}
        }

        // Decode proper
        if let Some(rgba) = bitmap.unpack_rgba(in_sys_info).ok() {
            println!("Successfully decoded \"{}\"", tex.name.as_str());
        }
    }
}

fn swap_image_bytes(data: &mut [u8]) {
    let mut tmp: u8;

    for d in data.chunks_exact_mut(8) {
        tmp = d[0];
        d[0] = d[1];
        d[1] = tmp;

        tmp = d[2];
        d[2] = d[3];
        d[3] = tmp;

        tmp = d[4];
        d[4] = d[5];
        d[5] = tmp;

        tmp = d[6];
        d[6] = d[7];
        d[7] = tmp;
    }
}