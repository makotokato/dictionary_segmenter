use crate::bytestrie::*;

// https://searchfox.org/mozilla-central/rev/8f08c21f093be1c1c42438697f8bca67af94fc77/intl/icu/source/common/brkeng.cpp#250

#[derive(Clone)]
pub struct DictionaryIterator<'a> {
    trie: BytesTrie<'a>,
    iter: &'a [u16],
    front_offset: usize,
    transform: u32,
}

impl<'a> Iterator for DictionaryIterator<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iter.len() <= self.front_offset {
            return None
        }

        let mut i = 0;
        let mut intermediate_length = 0;
        loop {
            if self.iter.len() <= i + self.front_offset {
                // Reach EOF
                self.front_offset = self.iter.len();
                return Some(self.front_offset);
            }
            let result = match i {
                0 => self
                    .trie
                    .first((self.iter[i + self.front_offset] as u32 - self.transform) as u8),
                _ => self
                    .trie
                    .next((self.iter[i + self.front_offset] as u32 - self.transform) as u8),
            };
            if result == BytesTrieResult::FinalValue {
                self.front_offset += i + 1;
                return Some(self.front_offset);
            }
            if result == BytesTrieResult::Intermediate {
                intermediate_length = i + 1;
            }
            if result == BytesTrieResult::NoMatch {
                break;
            }
            i += 1;
        }
        if intermediate_length > 0 {
            self.front_offset += intermediate_length;
            return Some(self.front_offset);
        }
        // no match
        self.front_offset = self.iter.len();
        None
    }
}

impl<'a> DictionaryIterator<'a> {
    pub fn new(dictionary: &'a [u8], offset: usize, transform: u32, input: &'a [u16]) -> Self {
        Self {
            trie: BytesTrie::new(dictionary, 0x90 + offset),
            iter: input,
            front_offset: 0,
            transform: transform,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::iter::*;

    #[test]
    fn iter_test() {
        const KHMER_DATA: &[u8; 445542] = include_bytes!("../data/khmerdict.dict");
        const KM_STR: [u16; 27] = [
            0x1797, 0x17b6, 0x179f, 0x17b6, 0x1781, 0x17d2, 0x1798, 0x17c2, 0x179a, 0x1797, 0x17b6,
            0x179f, 0x17b6, 0x1781, 0x17d2, 0x1798, 0x17c2, 0x179a, 0x1797, 0x17b6, 0x179f, 0x17b6,
            0x1781, 0x17d2, 0x1798, 0x17c2, 0x179a,
        ];
        let mut iterator = DictionaryIterator::new(KHMER_DATA, 0x20, 0x1780, &KM_STR);
        assert_eq!(iterator.next(), Some(9));
        assert_eq!(iterator.next(), Some(18));
        assert_eq!(iterator.next(), Some(27));

        const LAO_DATA: &[u8; 162620] = include_bytes!("../data/laodict.dict");
        const LO_STR: [u16; 21] = [
            0x0e9e, 0x0eb2, 0x0eaa, 0x0eb2, 0x0ea5, 0x0eb2, 0x0ea7, 0x0e9e, 0x0eb2, 0x0eaa, 0x0eb2,
            0x0ea5, 0x0eb2, 0x0ea7, 0x0e9e, 0x0eb2, 0x0eaa, 0x0eb2, 0x0ea5, 0x0eb2, 0x0ea7,
        ];
        let mut iterator = DictionaryIterator::new(LAO_DATA, 0x20, 0x0e80, &LO_STR);
        assert_eq!(iterator.next(), Some(4));
        assert_eq!(iterator.next(), Some(7));
        assert_eq!(iterator.next(), Some(11));
        assert_eq!(iterator.next(), Some(14));
        assert_eq!(iterator.next(), Some(18));
        assert_eq!(iterator.next(), Some(21));
    }
}
