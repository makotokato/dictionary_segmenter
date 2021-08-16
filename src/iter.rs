use crate::bytestrie::*;

// https://searchfox.org/mozilla-central/rev/8f08c21f093be1c1c42438697f8bca67af94fc77/intl/icu/source/common/brkeng.cpp#250
// 00000090 indexes
// 00000090+0x20(indexes[0]) = data

const KHMER_DATA: &[u8; 445542] = include_bytes!("../data/khmerdict.dict");

#[derive(Clone)]
pub struct DictionaryIterator<'a> {
    front_offset: usize,
    iter: &'a [u16],
    transform: u32,
}

impl<'a> Iterator for DictionaryIterator<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let mut trie = BytesTrie::new(KHMER_DATA, 0xb0);
        let mut i = 0;
        loop {
            let result = match i {
                0 => trie.first((self.iter[i + self.front_offset] as u32 - self.transform) as u8),
                _ => trie.next((self.iter[i + self.front_offset] as u32 - self.transform) as u8),
            };
            if result == BytesTrieResult::FinalValue {
                self.front_offset += i + 1;
                return Some(self.front_offset);
            }
            if result == BytesTrieResult::NoMatch {
                break;
            }
            i += 1;
        }
        None
    }
}

impl<'a> DictionaryIterator<'a> {
    pub fn new(input: &'a [u16]) -> Self {
        Self {
            front_offset: 0,
            iter: input,
            transform: 0x1780,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::bytestrie::*;
    use crate::iter::*;

    #[test]
    fn iter_test() {
        const STR: [u16; 27] = [
            0x1797, 0x17b6, 0x179f, 0x17b6, 0x1781, 0x17d2, 0x1798, 0x17c2, 0x179a, 0x1797, 0x17b6,
            0x179f, 0x17b6, 0x1781, 0x17d2, 0x1798, 0x17c2, 0x179a, 0x1797, 0x17b6, 0x179f, 0x17b6,
            0x1781, 0x17d2, 0x1798, 0x17c2, 0x179a,
        ];
        let mut iterator = DictionaryIterator::new(&STR);
        assert_eq!(iterator.next(), Some(9));
        assert_eq!(iterator.next(), Some(18));
        assert_eq!(iterator.next(), Some(27));
    }
}
