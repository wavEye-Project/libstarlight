/*
.slpatch and hexpatching handler
*/
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::HashMap,
    fs,
    io::{Error, Read, Seek, SeekFrom},
    ops::Deref
};
use windows::Win32::System::SystemInformation::*;

#[derive(Serialize, Deserialize)]
pub struct PatchRegex(#[serde(
    serialize_with = "serialize_regex",
    deserialize_with = "deserialize_regex"
)] Regex);
impl PatchRegex {
    pub fn new(pattern: Regex) -> Self {
        PatchRegex(pattern)
    }
}
impl Deref for PatchRegex {
    type Target = Regex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn deserialize_regex<'de, D>(deserializer: D) -> Result<Regex, D::Error>
where
    D: Deserializer<'de>,
{
    let old_pattern: String = Deserialize::deserialize(deserializer)?;
    let pattern = old_pattern.replace(" ", "").to_lowercase();
    Regex::new(format!("(?m){}", pattern).as_str())
        .map_err(|_| serde::de::Error::custom("Invalid regex pattern"))
}
fn serialize_regex<S>(regex: &Regex, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&regex.to_string())
}
pub type PatchData = Vec<(PatchRegex, String)>;

#[derive(Serialize, Deserialize)]
pub struct Patch {
    pub module: String,
    pub patterns: HashMap<String, PatchData>,
}

#[derive(Serialize, Deserialize)]
pub struct PatchRoot {
    pub name: String,
    pub version: String,
    pub process: String,
    pub patches: Vec<Patch>,
}

pub fn open_slpatch(path: &str) -> Result<PatchRoot, Error> {
    serde_json::from_str(&fs::read_to_string(path)?).map_err(Into::into)
}

pub fn check_machine(filename: &str) -> Result<String, String> {
    let mut file = fs::File::open(filename).map_err(|_| "Failed to open file".to_string())?;

    let mut buffer = [0; 4];
    file.seek(SeekFrom::Start(0x3C)).ok();
    file.read_exact(&mut buffer).ok();
    let coff_offset = u32::from_le_bytes(buffer);

    file.seek(SeekFrom::Start(coff_offset as u64)).ok();
    file.read_exact(&mut buffer).ok();
    file.read_exact(&mut buffer).ok();
    let machine = u16::from_le_bytes([buffer[0], buffer[1]]);
    match IMAGE_FILE_MACHINE(machine) {
        IMAGE_FILE_MACHINE_AMD64 => Ok("amd64".to_string()),
        IMAGE_FILE_MACHINE_I386 => Ok("i386".to_string()),
        IMAGE_FILE_MACHINE_ARM | IMAGE_FILE_MACHINE_ARMNT => Ok("arm".to_string()),
        IMAGE_FILE_MACHINE_ARM64 => Ok("arm64".to_string()),
        _ => Err("Unsupported machine header".to_string()),
    }
}


// TODO: Fix performance
pub fn patch_module(patches: &PatchData, content: &Vec<u8>) -> Result<Vec<u8>, String> {
    let mut hexdata: String = hex::encode(content);
    for (regex, _0) in patches {
        let subst = _0.replace(" ", "").to_lowercase();
        hexdata = regex.replace_all(&hexdata, subst).to_string();
    }
    hex::decode(&hexdata).map_err(|_| "Patched data is corrupt".to_string())
}
