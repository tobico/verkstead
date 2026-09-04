//! A name for a directory that is somewhere else, on the platform whose links
//! are not the Unixes'.
//!
//! **A directory junction**, which is a reparse point on an empty directory
//! saying where to go instead. Windows has three things that could stand here
//! and this is the only one Verkstead can use: a directory symbolic link and a
//! file symbolic link both need `SeCreateSymbolicLinkPrivilege`, which an
//! administrator has and a per-user install has not, and which a machine that
//! is not in developer mode will not lend; a junction needs nothing at all. See
//! ADR-0014, where the whole of that is argued.
//!
//! **Written by hand, because nothing in the standard library writes one.**
//! `std::os::windows::fs::symlink_dir` makes the kind that needs the privilege.
//! So the reparse point is built as the filesystem reads one and set with
//! `FSCTL_SET_REPARSE_POINT`, which is the documented way and the only one.
//!
//! A module of its own rather than a `cfg` inside [`super::open`], which is
//! built on every platform and has nothing else in it that one platform has to
//! itself. What the arm compiled for a Unix does is written below, and what it
//! is for is the suite: the description a Windows session is rendered from is
//! portable, and only the boundary is not.

use std::io;
use std::path::Path;

/// A junction at `inside` pointing at `host`.
///
/// `inside` must not be there yet — a junction is made *as* the directory
/// rather than written onto one somebody else made. Whoever calls this has
/// already taken away whatever was at the name; see [`super::open::joined`].
#[cfg(windows)]
pub(super) fn at(host: &Path, inside: &Path) -> io::Result<()> {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    use windows_sys::Win32::Foundation::{CloseHandle, GENERIC_WRITE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::FSCTL_SET_REPARSE_POINT;

    // A junction is an empty directory with a reparse point on it, so the
    // directory comes first and the point is written into it.
    std::fs::create_dir(inside)?;

    let point = pointing(host)?;

    let name: Vec<u16> = inside.as_os_str().encode_wide().chain(Some(0)).collect();

    // Opened as the reparse point rather than through it — a directory being a
    // thing this platform will only open with the backup flag on.
    //
    // Safety: the name is NUL-terminated, and neither of the two pointers this
    // passes none of is read.
    let directory = unsafe {
        CreateFileW(
            name.as_ptr(),
            GENERIC_WRITE,
            0,
            ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            ptr::null_mut(),
        )
    };

    if directory == INVALID_HANDLE_VALUE {
        let why = io::Error::last_os_error();
        let _ = std::fs::remove_dir(inside);

        return Err(why);
    }

    let mut returned = 0u32;

    // Safety: the buffer is the structure the control code is documented to
    // take, its length is the header plus what the structure itself says
    // follows it, and no output buffer is asked for.
    let set = unsafe {
        DeviceIoControl(
            directory,
            FSCTL_SET_REPARSE_POINT,
            ptr::from_ref(&point).cast::<c_void>(),
            u32::try_from(HEADER + usize::from(point.data_length)).unwrap_or(u32::MAX),
            ptr::null_mut(),
            0,
            &raw mut returned,
            ptr::null_mut(),
        )
    };

    // Read before the handle is closed, which is a call with an error of its
    // own to set.
    let why = (set == 0).then(io::Error::last_os_error);

    // Safety: the handle is this function's own and is not used again.
    unsafe { CloseHandle(directory) };

    if let Some(why) = why {
        // A directory with no reparse point on it is not a junction, and one
        // left standing where the account was to be found is worse than the
        // name not being there at all: a session would read an empty account
        // rather than none.
        let _ = std::fs::remove_dir(inside);

        return Err(why);
    }

    Ok(())
}

/// The reparse point that says `host`, as the filesystem reads one.
///
/// Two names in it, which is what a mount point carries: the **substitute
/// name**, which is what the filesystem follows and is written in the object
/// manager's own spelling — `\??\C:\…` rather than the `\\?\C:\…` a human
/// types — and the **print name**, which is what a tool showing the link to
/// somebody displays. Each is NUL-terminated inside the buffer and neither
/// terminator is counted in its length, which is how `mklink /J` writes one and
/// what every reader expects.
///
/// `host` is resolved first. A reparse point is followed by the filesystem
/// rather than by a program, so what is in it has to be absolute and have
/// nothing left to work out; `canonicalize` is what answers that on this
/// platform, and hands the `\\?\` spelling back. Where it cannot answer — a
/// path with nothing at it — the name as it was written goes in, which is a
/// junction to nowhere rather than no junction at all.
#[cfg(windows)]
fn pointing(host: &Path) -> io::Result<MountPoint> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    let real = std::fs::canonicalize(host).unwrap_or_else(|_| host.to_owned());
    let real: Vec<u16> = real.as_os_str().encode_wide().collect();

    let verbatim: Vec<u16> = OsStr::new(VERBATIM).encode_wide().collect();
    let printed = real.strip_prefix(verbatim.as_slice()).unwrap_or(&real);

    let substitute: Vec<u16> = OsStr::new(OBJECT)
        .encode_wide()
        .chain(printed.iter().copied())
        .collect();

    // The two names and the NUL after each, in words.
    let words = substitute.len() + 1 + printed.len() + 1;

    if words > WORDS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is too long a name for a junction", host.display()),
        ));
    }

    let mut point = MountPoint {
        tag: MOUNT_POINT,
        data_length: 0,
        reserved: 0,
        substitute_offset: 0,
        substitute_length: 0,
        print_offset: 0,
        print_length: 0,
        path: [0; WORDS],
    };

    point.path[..substitute.len()].copy_from_slice(&substitute);
    point.path[substitute.len() + 1..][..printed.len()].copy_from_slice(printed);

    // Offsets and lengths are in bytes rather than in words, which is the one
    // thing about this structure that is easy to write wrong.
    point.substitute_length = words_of(substitute.len());
    point.print_offset = words_of(substitute.len() + 1);
    point.print_length = words_of(printed.len());
    point.data_length = words_of(words) + u16::try_from(NAMES).unwrap_or(u16::MAX);

    Ok(point)
}

/// How many bytes `words` words come to, as this structure counts.
#[cfg(windows)]
fn words_of(words: usize) -> u16 {
    u16::try_from(words * 2).unwrap_or(u16::MAX)
}

/// The reparse point a junction is, laid out as `REPARSE_DATA_BUFFER` is with
/// its mount-point half filled in.
///
/// Written out here rather than taken from `windows-sys`, whose one is in the
/// driver namespace and is a union over the three shapes a reparse point can
/// have — which is more to reach through than to write, for a structure that is
/// four fixed words and two names.
///
/// The names are a fixed array rather than a tail, because the buffer handed to
/// a control code has to be aligned as the structure is and a `Vec<u8>` is not.
/// [`REPARSE_MAX`] is what the filesystem will take, so nothing here can be too
/// small for a name it will accept.
#[cfg(windows)]
#[repr(C)]
struct MountPoint {
    tag: u32,
    data_length: u16,
    reserved: u16,
    substitute_offset: u16,
    substitute_length: u16,
    print_offset: u16,
    print_length: u16,
    path: [u16; WORDS],
}

/// What a mount point's reparse point is tagged with.
///
/// Written out rather than imported: `windows-sys` carries it in
/// `Win32_System_SystemServices`, which is thousands of constants for this one
/// — and it is a documented protocol number rather than an API, so what it
/// costs to have it here is that it is here.
#[cfg(windows)]
const MOUNT_POINT: u32 = 0xa000_0003;

/// The largest reparse point the filesystem will take, and how it is divided:
/// the tag, the length and the reserved word in front, the two offsets and the
/// two lengths after them, and the names in what is left.
#[cfg(windows)]
const REPARSE_MAX: usize = 16 * 1024;
#[cfg(windows)]
const HEADER: usize = 8;
#[cfg(windows)]
const NAMES: usize = 8;
#[cfg(windows)]
const WORDS: usize = (REPARSE_MAX - HEADER - NAMES) / 2;

/// The spelling a path is followed under, and the one `canonicalize` hands one
/// back in — the same name, said to the object manager and said to the Win32
/// layer over it.
#[cfg(windows)]
const OBJECT: &str = r"\??\";
#[cfg(windows)]
const VERBATIM: &str = r"\\?\";

/// And what the arm compiled for a Unix makes, which is the nearest thing that
/// machine has to say it with.
///
/// **Nothing in production reaches this.** The rendering it belongs to is the
/// Windows one, and [`crate::platform::Platform::HERE`] is never Windows on a
/// Unix. What it is for is the suite: what a Windows session's profile comes
/// out holding is a question the description can answer on any machine, and a
/// test that could only be run on one would be a test nobody here runs.
#[cfg(unix)]
pub(super) fn at(host: &Path, inside: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(host, inside)
}
