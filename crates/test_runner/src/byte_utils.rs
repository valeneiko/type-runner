#[inline]
pub const fn trim_space_start(bytes: &[u8]) -> &[u8] {
    let mut bytes = bytes;
    while let [first, rest @ ..] = bytes {
        if *first == b' ' {
            bytes = rest;
        } else {
            break;
        }
    }

    bytes
}

#[inline]
pub const fn trim_space_end(bytes: &[u8]) -> &[u8] {
    let mut bytes = bytes;
    while let [rest @ .., last] = bytes {
        if *last == b' ' {
            bytes = rest;
        } else {
            break;
        }
    }

    bytes
}

#[inline]
pub const fn trim_space(bytes: &[u8]) -> &[u8] {
    trim_space_end(trim_space_start(bytes))
}
