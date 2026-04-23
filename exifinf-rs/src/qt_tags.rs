//! Curated quick lookups for ISO BMFF / QuickTime (subset of ExifTool semantics).

/// (friendly `File:FileType` string, MIME type) from major ftyp brand and compatible list.
pub fn ftyp_to_file_mime(major: &[u8; 4], compat: &[[u8; 4]]) -> (String, String) {
    if let Some((t, m)) = brand_tuple(major) {
        return (t, m);
    }
    for c in compat {
        if let Some((t, m)) = brand_tuple(c) {
            return (t, m);
        }
    }
    ("MP4".to_string(), "video/mp4".to_string())
}

fn as_brand_str(brand: &[u8; 4]) -> String {
    String::from_utf8_lossy(brand)
        .trim()
        .trim_end_matches(char::from(0))
        .to_string()
}

fn brand_tuple(brand: &[u8; 4]) -> Option<(String, String)> {
    let s = as_brand_str(brand);
    match s.as_str() {
        "qt" | "MOV" => Some(("MOV".to_string(), "video/quicktime".to_string())),
        "M4A" | "M4B" | "F4A" | "F4B" | "M4P" | "F4P" | "isom" | "iso2" | "mp41" | "mp42"
        | "avc1" | "dash" | "dsms" | "dby1" | "Cdmv" | "Cmov" => {
            Some(("MP4".to_string(), "video/mp4".to_string()))
        }
        "M4V" | "F4V" | "mmp4" => Some(("M4V".to_string(), "video/x-m4v".to_string())),
        "heic" | "mif1" | "heix" | "heis" | "hevm" | "mif0" | "heio" | "msf0" | "ihev" => {
            Some(("HEIC".to_string(), "image/heic".to_string()))
        }
        "3gp" | "3g2" | "3G2" | "3G6" | "3ge" | "3gg" | "3gr" | "3gs" | "3gh" | "3gp1" | "3gp2"
        | "3gp4" | "3gp5" | "3gp6" | "3g24" | "3gg6" | "3gs7" | "3gs8" | "3gs9" | "3gdd" | "3g2a" => {
            Some(("3GPP".to_string(), "video/3gpp".to_string()))
        }
        "jp2" | "jpx" | "jpm" => Some(("JP2".to_string(), "image/jp2".to_string())),
        s if s.len() == 4 && s.starts_with('3') => Some(("3GPP".to_string(), "video/3gpp".to_string())),
        _ => None,
    }
}

const NAM: [u8; 4] = [0xA9, b'n', b'a', b'm'];
const NAM2: [u8; 4] = *b"nam ";
const ART: [u8; 4] = [0xA9, b'A', b'R', b'T'];
const AUT: [u8; 4] = [0xA9, b'a', b'u', b't'];
const ARTS: [u8; 4] = *b"ART ";
const WRT: [u8; 4] = [0xA9, b'w', b'r', b't'];
const ALB: [u8; 4] = [0xA9, b'a', b'l', b'b'];
const ALBM: [u8; 4] = *b"albm";
const DAY: [u8; 4] = [0xA9, b'd', b'a', b'y'];
const CMT: [u8; 4] = [0xA9, b'c', b'm', b't'];
const DESC: [u8; 4] = *b"desc";
const CPY: [u8; 4] = [0xA9, b'c', b'p', b'y'];
const CPY2: [u8; 4] = *b"cpy ";
const COPY: [u8; 4] = *b"copy";
const XYZ: [u8; 4] = [0xA9, b'x', b'y', b'z'];
const XYZ2: [u8; 4] = *b"xyz ";
const MAK: [u8; 4] = [0xA9, b'm', b'a', b'k'];
const MOD: [u8; 4] = [0xA9, b'm', b'o', b'd'];
const SWR: [u8; 4] = [0xA9, b's', b'w', b'r'];
const TOOL: [u8; 4] = *b"tool";
const TOO: [u8; 4] = *b"too ";
const ATOO: [u8; 4] = [0xA9, b't', b'o', b'o'];
const SOFT: [u8; 4] = *b"soft";
const GEN: [u8; 4] = [0xA9, b'g', b'e', b'n'];
const AART: [u8; 4] = *b"aART";
const CPRT: [u8; 4] = *b"cprt";
const ARTS2: [u8; 4] = *b"arts";
const PERF: [u8; 4] = *b"perf";
const TRKN: [u8; 4] = *b"trkn";
const DISK: [u8; 4] = *b"disk";
const COVR: [u8; 4] = *b"covr";
const CPIL: [u8; 4] = *b"cpil";
const PGAP: [u8; 4] = *b"pgap";
const GEID: [u8; 4] = *b"geID";
const GNRE: [u8; 4] = *b"gnre";

/// Map classic `udta` 4cc to (group, name).
pub fn classic_udta_atom(kind: &[u8; 4]) -> Option<(&'static str, &'static str)> {
    let k = *kind;
    if k == NAM || k == NAM2 {
        return Some(("QuickTime", "Title"));
    }
    if k == ART || k == AUT || k == ARTS || k == ARTS2 || k == PERF {
        return Some(("QuickTime", "Artist"));
    }
    if k == WRT {
        return Some(("QuickTime", "Composer"));
    }
    if k == ALB || k == ALBM {
        return Some(("QuickTime", "Album"));
    }
    if k == DAY {
        return Some(("QuickTime", "DateTimeOriginal"));
    }
    if k == CMT || k == DESC {
        return Some(("QuickTime", "Comment"));
    }
    if k == CPY || k == CPY2 || k == COPY {
        return Some(("QuickTime", "Copyright"));
    }
    if k == XYZ || k == XYZ2 {
        return Some(("QuickTime", "GPSCoordinates"));
    }
    if k == MAK {
        return Some(("IFD0", "Make"));
    }
    if k == MOD {
        return Some(("IFD0", "Model"));
    }
    if k == SWR || k == TOOL || k == TOO || k == ATOO || k == SOFT {
        return Some(("IFD0", "Software"));
    }
    if k == GEN {
        return Some(("QuickTime", "Genre"));
    }
    None
}

/// iTunes/ilst sub-box 4cc → (group, name) for `data` value.
pub fn ilst_data_atom(kind: &[u8; 4]) -> Option<(&'static str, &'static str)> {
    let k = *kind;
    if k == NAM || k == NAM2 {
        return Some(("QuickTime", "Title"));
    }
    if k == ART || k == AART || k == ARTS {
        return Some(("QuickTime", "Artist"));
    }
    if k == ALB || k == ALBM {
        return Some(("QuickTime", "Album"));
    }
    if k == DAY {
        return Some(("QuickTime", "DateTimeOriginal"));
    }
    if k == CMT || k == DESC {
        return Some(("QuickTime", "Comment"));
    }
    if k == CPY || k == CPRT || k == COPY {
        return Some(("QuickTime", "Copyright"));
    }
    if k == WRT {
        return Some(("QuickTime", "Composer"));
    }
    if k == MAK {
        return Some(("IFD0", "Make"));
    }
    if k == MOD {
        return Some(("IFD0", "Model"));
    }
    if k == TOO || k == TOOL || k == SWR {
        return Some(("IFD0", "Software"));
    }
    if k == GEN {
        return Some(("QuickTime", "Genre"));
    }
    if k == TRKN || k == DISK || k == COVR || k == CPIL || k == PGAP || k == GEID || k == GNRE {
        return None;
    }
    None
}

/// Apple `meta/keys` reverse-DNS key → (group, name).
pub fn apple_meta_key(key: &str) -> Option<(&'static str, &'static str)> {
    match key {
        "com.apple.quicktime.make" => Some(("IFD0", "Make")),
        "com.apple.quicktime.model" => Some(("IFD0", "Model")),
        "com.apple.quicktime.software" => Some(("IFD0", "Software")),
        "com.apple.quicktime.creationdate" => Some(("QuickTime", "CreateDate")),
        "com.apple.quicktime.location.name" => Some(("QuickTime", "LocationName")),
        "com.apple.quicktime.location.body" => Some(("QuickTime", "LocationBody")),
        "com.apple.quicktime.location.note" => Some(("QuickTime", "LocationNote")),
        "com.apple.quicktime.location.ISO6709" => Some(("QuickTime", "GPSCoordinates")),
        "com.apple.quicktime.description" => Some(("QuickTime", "Description")),
        "com.apple.quicktime.title" => Some(("QuickTime", "Title")),
        "com.apple.quicktime.artist" => Some(("QuickTime", "Artist")),
        "com.apple.quicktime.album" => Some(("QuickTime", "Album")),
        "com.apple.quicktime.copyright" => Some(("QuickTime", "Copyright")),
        "com.apple.quicktime.owner" => Some(("QuickTime", "Owner")),
        "com.apple.quicktime.author" => Some(("QuickTime", "Author")),
        "com.apple.quicktime.displayname" => Some(("QuickTime", "DisplayName")),
        "com.apple.quicktime.category" => Some(("QuickTime", "Category")),
        "com.apple.quicktime.keywords" => Some(("QuickTime", "KeywordList")),
        _ => None,
    }
}
