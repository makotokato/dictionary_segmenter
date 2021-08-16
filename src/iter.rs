use crate::bytestrie::*;

// https://searchfox.org/mozilla-central/rev/8f08c21f093be1c1c42438697f8bca67af94fc77/intl/icu/source/common/brkeng.cpp#250
// 00000090 indexes
// 00000090+0x20(indexes[0]) = data

//const DATA: &[u8; 445542] = include_bytes!("../data/khmerdict.dict");

//fn load() {
    //let trie = BytesTrie::new(DATA, 0);
//}

#[cfg(test)]
mod tests {
    use crate::bytestrie::*;

    #[test]
    fn match_text() {
        const DATA: &[u8; 445542] = include_bytes!("../data/khmerdict.dict");
        const STR: [u16; 27] = [ 0x1797, 0x17b6, 0x179f, 0x17b6, 0x1781, 0x17d2, 0x1798, 0x17c2, 0x179a, 0x1797, 0x17b6, 0x179f, 0x17b6, 0x1781, 0x17d2, 0x1798, 0x17c2, 0x179a, 0x1797, 0x17b6, 0x179f, 0x17b6, 0x1781, 0x17d2, 0x1798, 0x17c2, 0x179a ];

        let mut trie = BytesTrie::new(DATA, 0);
        let mut i = 0;
        let mut found = false;
        loop {
            let result;
            if i == 0 {
                result = trie.first((STR[i] - 0x1780) as u8);
            } else {
                result = trie.next((STR[i] - 0x1780) as u8);
            }
            match result {
               BytesTrieResult::NoValue => println!("{}, NoValue", i),
               BytesTrieResult::NoMatch => println!("{}, NoMatch", i),
               BytesTrieResult::FinalValue => println!("{}, FinalValue", i),
               BytesTrieResult::Intermediate => println!("{}, Intermediate", i),
              
            }
            if result == BytesTrieResult::FinalValue || result == BytesTrieResult::Intermediate {
               //
              if result == BytesTrieResult::FinalValue {
                i += 1;
                found = true;
                break;
              }
            }
            if result == BytesTrieResult::NoMatch {
                break;
            }
            i += 1;
        }
        println!("{}", i);
    }
}
