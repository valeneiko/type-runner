use memchr::{Memchr, memchr_iter};

pub(super) struct LineIter<'a> {
    data: &'a [u8],
    iter: Memchr<'a>,
    pub line_start: usize,
    line_idx: usize,
}

impl<'a> LineIter<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, iter: memchr_iter(b'\n', data), line_start: 0, line_idx: 0 }
    }
}

impl<'a> Iterator for LineIter<'a> {
    type Item = (usize, usize, &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        match self.line_start.cmp(&self.data.len()) {
            std::cmp::Ordering::Less => {
                let eol = self.iter.next().unwrap_or_else(|| self.data.len() - 1);
                let line = &self.data[self.line_start..=eol];
                let line_end = line.len()
                    - if line.len() >= 2 && line[line.len() - 2] == b'\r' {
                        2
                    } else {
                        usize::from(!line.is_empty() && line[line.len() - 1] == b'\n')
                    };

                let result = (self.line_idx, self.line_start, &line[..line_end]);
                self.line_start = eol + 1;
                self.line_idx += 1;

                Some(result)
            }
            std::cmp::Ordering::Equal => {
                let result = (self.line_idx, self.line_start, &b""[..]);
                self.line_start += 1;
                self.line_idx += 1;
                Some(result)
            }
            std::cmp::Ordering::Greater => None,
        }
    }
}
