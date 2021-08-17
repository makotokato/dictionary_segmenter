use crate::bytestrie::*;
use crate::trie::*;
use crate::ucharstrie::*;

const TRIE_TYPE_BYTES: u32 = 0;
const TRIE_TYPE_UCHARS: u32 = 1;
const TRIE_TYPE_MASK: u32 = 7;

const TRANSFORM_TYPE_OFFSET: u32 = 0x1000000;
const TRANSFORM_TYPE_MASK: u32 = 0x7f000000;
const TRANSFORM_OFFSET_MASK: u32 = 0x1fffff;

#[repr(C)]
struct TrieHeader {
    pub icu_header: [u8; 0x90],
    pub trie_offset: u32,
    pub reserved1: u32,
    pub reserved2: u32,
    pub total_size: u32,
    pub trie_type: u32,
    pub transform: u32,
    pub reserved6: u32,
    pub reserved7: u32,
}

#[derive(Clone)]
pub struct DictionaryIterator<'a> {
    trie: Box<dyn Trie>,
    iter: &'a [u16],
    front_offset: usize,
    transform: u32,
    dictionary: &'a [u8],
}

impl<'a> Iterator for DictionaryIterator<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iter.len() <= self.front_offset {
            return None;
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
                0 => self.trie.first(
                    self.dictionary,
                    self.transform(self.iter[i + self.front_offset]),
                ),
                _ => self.trie.next(
                    self.dictionary,
                    self.transform(self.iter[i + self.front_offset]),
                ),
            };
            if result == TrieResult::FinalValue {
                self.front_offset += i + 1;
                return Some(self.front_offset);
            }
            if result == TrieResult::Intermediate {
                intermediate_length = i + 1;
                // continue for better string
            }
            if result == TrieResult::NoMatch {
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
    pub fn new(dictionary: &'a [u8], input: &'a [u16]) -> Self {
        if dictionary.len() < core::mem::size_of::<TrieHeader>() {
            panic!("too small data");
        }
        let header = unsafe { &*(dictionary as *const [u8] as *const TrieHeader) };
        let trie_type = header.trie_type & TRIE_TYPE_MASK;
        match trie_type {
            TRIE_TYPE_BYTES => Self {
                trie: Box::new(BytesTrie::new(0x90 + header.trie_offset as usize)),
                iter: input,
                front_offset: 0,
                transform: header.transform,
                dictionary: dictionary,
            },
            TRIE_TYPE_UCHARS => Self {
                trie: Box::new(UCharsTrie::new(0x90 + header.trie_offset as usize)),
                iter: input,
                front_offset: 0,
                transform: header.transform,
                dictionary: dictionary,
            },
            _ => panic!("unknown type"),
        }
    }

    fn transform(&self, c: u16) -> i32 {
        if self.transform & TRANSFORM_TYPE_MASK == TRANSFORM_TYPE_OFFSET {
            match c {
                0x200c => 0xfe,
                0x200d => 0xff,
                _ => {
                    let delta = (c as u32 - (self.transform & TRANSFORM_OFFSET_MASK)) as i32;
                    if delta < 0 || delta > 0xfd {
                        -1
                    } else {
                        delta
                    }
                }
            }
        } else {
            c as i32
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::dictionary_iter::*;
    const KHMER_DATA: &[u8; 445542] = include_bytes!("../data/khmerdict.dict");
    const LAO_DATA: &[u8; 162620] = include_bytes!("../data/laodict.dict");
    const CJ_DATA: &[u8; 2003566] = include_bytes!("../data/cjdict.dict");

    #[test]
    fn trie_iter_test() {
        const KM_STR: [u16; 27] = [
            0x1797, 0x17b6, 0x179f, 0x17b6, 0x1781, 0x17d2, 0x1798, 0x17c2, 0x179a, 0x1797, 0x17b6,
            0x179f, 0x17b6, 0x1781, 0x17d2, 0x1798, 0x17c2, 0x179a, 0x1797, 0x17b6, 0x179f, 0x17b6,
            0x1781, 0x17d2, 0x1798, 0x17c2, 0x179a,
        ];
        let mut iterator = DictionaryIterator::new(KHMER_DATA, &KM_STR);
        assert_eq!(iterator.next(), Some(9));
        assert_eq!(iterator.next(), Some(18));
        assert_eq!(iterator.next(), Some(27));

        const LO_STR: [u16; 21] = [
            0x0e9e, 0x0eb2, 0x0eaa, 0x0eb2, 0x0ea5, 0x0eb2, 0x0ea7, 0x0e9e, 0x0eb2, 0x0eaa, 0x0eb2,
            0x0ea5, 0x0eb2, 0x0ea7, 0x0e9e, 0x0eb2, 0x0eaa, 0x0eb2, 0x0ea5, 0x0eb2, 0x0ea7,
        ];
        let mut iterator = DictionaryIterator::new(LAO_DATA, &LO_STR);
        assert_eq!(iterator.next(), Some(4));
        assert_eq!(iterator.next(), Some(7));
        assert_eq!(iterator.next(), Some(11));
        assert_eq!(iterator.next(), Some(14));
        assert_eq!(iterator.next(), Some(18));
        assert_eq!(iterator.next(), Some(21));

        const J_STR: [u16; 8] = [
            0x713c, 0x8089, 0x5b9a, 0x98df, 0x3092, 0x98df, 0x3079, 0x308b,
        ];
        let mut iterator = DictionaryIterator::new(CJ_DATA, &J_STR);
        assert_eq!(iterator.next(), Some(2));
        assert_eq!(iterator.next(), Some(4));
        assert_eq!(iterator.next(), Some(5));
        assert_eq!(iterator.next(), Some(8));
    }
}
