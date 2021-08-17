#![feature(test)]

extern crate test;

#[cfg(test)]
mod bench {
    use test::Bencher;
    use bytestrie::DictionaryIterator;

    const TEST_KM_STR: &str = "ភាសាខ្មែរភាសាខ្មែរភាសាខ្មែរ";
    const KHMER_DATA: &[u8; 445542] = include_bytes!("../data/khmerdict.dict");

    #[bench]
    fn linebreak_iter_utf16(b: &mut Bencher) {
        let utf16: Vec<u16> = TEST_KM_STR.encode_utf16().map(|x| x).collect();
        b.iter(|| DictionaryIterator::new(KHMER_DATA, &utf16).count())
    }
}
