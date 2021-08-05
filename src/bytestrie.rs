// 00..0f: Branch node. If node!=0 then the length is node+1, otherwise
// the length is one more than the next unit.

// 10..1f: Linear-match node, match 1..16 units and continue reading the next node.
const MIN_LINEAR_MATCH: u8 = 0x10;
const MAX_LINEAR_MATCH_LENGTH: u8 = 0x10;

const MIN_VALUE_LEAD: u8 = 0x20;

// A final-value node has bit 0 set.
const VALUE_IS_FINAL: u8 = 1;

struct BytesTrieResult {
    state: u32,
}

impl BytesTrieResult {
    pub fn no_value() -> Self {
        Self { state: NO_VALUE }
    }

    pub fn no_match() -> Self {
        Self { state: NO_MATCH }
    }
}

struct BytesTrie {
    chars_: u8,
    pos_: Option<usize>,
    root_: usize,
    remaining_match_length: Option<usize>,
}

impl ByteTrie {
    pub fn new(trieChars: &str, offset: usize) -> Self {
        Self {
            chars_: x,
            pos_: Some(offset),
            root_: offset,
            remaining_match_length_: None,
        }
    }

    pub fn reset(mut self) {
        self.pos_ = Some(self.root_);
        self.remaining_match_length_ = None;
    }

    pub fn current(self) -> BytesTrieResult {
        if self.pos_.is_none() {
            return CharTrieResult::no_match();
        }

        let pos = self.pos_.unwrap();
        let node = self.bytes_[pos] as u8;
        if self.remaining_match_length_.is_none() && node >= MIN_VALUE_LEAD {
            return self.valueResult_[node >> 15];
        }

        CharTrieResult::no_value()
    }

    // Traverses the trie from the initial state for this input char.
    // Equivalent to reset() then next(inUnit)
    pub fn first(mut self, in_unit: u8) -> BytesTrieResult {
        self.remaining_match_length_ = None;
        self.nextImpl(self.root_, in_unit)
    }

    // Traverses the trie from the current state for this input char.
    pub fn next(mut self, in_unit: u8) -> BytesTrieResult {
        if self.pos_.is_none() {
            return BytesTrieResult::no_match();
        }

        let pos = self.pos_.unwrap();
        if let Some(length) = self.remainingMatchLength_ {
            // Remaining part of a linear-match node
            pos += 1;
            if in_unit == self.bytes_[pos - 1] {
                if length == 0 {
                    self.remainingMatchLength_ = None;
                    self.pos_ = Some(pos);
                    let node = self.bytes_[pos];
                    if node >= MIN_VALUE_LEAD {
                        return self.valueResults_[node & VALUE_IS_FINAL];
                    }
                    // no value
                    return CharTrieResult::no_value();
                } else {
                    self.remainingMatchLength_ = Some(length);
                    self.pos_ = Some(pos);
                    // no value
                    return BytesTrieResult::no_value();
                }
                self.stop();
                // no match
                return BytesTrieResult::no_match();
            }
        }
        self.nextImpl(pos, in_unit)
    }

    // Traverses the trie from the current state for this string.
    // Equivalent to
    pub fn next(mut self, s: &u8, index: usize, limit: usize) -> BytesTrieResult {
        if index >= limit {
            // Empty index.
            return self.current();
        }
        if self.pos_.is_none() {
            return BytesTrieResult::no_match();
        }

        let mut index = index;
        let pos = self.pos_.unwrap();
        let length = self.remaining_match_length_;

        loop {
            let in_unit;
            loop {
                if index == limit {
                    self.remaing_match_length_ = length;
                    self.pos_ = pos;
                    if length.is_none() {
                        let node = self.bytes_[pos];
                        if node >= MIN_VALUE_LEAD {
                            return self.valueResults_[node & VALUE_IS_FINAL];
                        }
                    }
                    // no value
                    return BytesTrieResult::no_value();
                }
                in_unit = s[index];
                index += 1;
                if length.is_none() {
                    self.remainingMatchLength_ = None;
                    break;
                }

                if in_unit != self.bytes_[pos] {
                    stop();
                    return BytesTrieResult::no_match();
                }
                pos += 1;
                if length.unwrap() == 0 {
                    length = None;
                } else {
                    length = Some(length.unwrap() - 1);
                }
            }
            loop {
                let node = self.bytes_[pos];
                pos += 1;
                if node < MIN_LINEAR_MATCH {
                    let result = self.branchNext(pos, node, in_unit);
                    if result.is_no_match() {
                        return BytesTrieResult::no_match();
                    }
                    // Fetch the next input byte, if there is one.
                    if index == limit {
                        return result;
                    }
                    if result.is_final_value() {
                        // No further matching bytes
                        self.stop();
                        return BytesTrieResult::no_match();
                    }
                    in_unit = s[index];
                    index += 1;
                    pos = self.pos_; // branchNext() advanced pos and wrote it to self.pos_;
                } else if node < MIN_VALUE_LEAD {
                    length = node - MIN_LINEAR_MATCH;
                    if in_unit != self.bytes_[pos] {
                        self.stop();
                        return BytesTrieResult::no_match();
                    }
                    pos += 1;
                    if length.unwrap() == 0 {
                        length = None;
                    } else {
                        length = Some(length.unwrap() - 1);
                    }
                    break;
                } else if (node & VALUE_IS_FINAL) != 0 {
                    // No further matching bytes
                    self.stop();
                    return BytesTrieResult::no_match();
                } else {
                    // Skip intermediate value.
                    pos = self.skipValue(pos, node);
                }
            }
        }
    }

    fn branchNext(self, pos: usize, length: usize, in_unit: u16) -> CharTrieResult {
        let mut pos = pos;
        if length == 0 {
            length = self.chars_.charAt(pos);
            pos += 1;
        }
        length += 1;

        // The length of the branch is the number of units to select from.
        // The data structure encodes a binary search.
        while length > MAX_BRANCH_LINEAR_SUB_NODE_LENGTH {
            pos += 1;
            if in_unit < self.chars_.chatAt(pos - 1) {
                length >>= 1;
                pos = self.jumpByDelta(self.chars_, pos);
            } else {
                length = length - (length >> 1);
                pos = self.skipDelta(self.chars_, pos);
            }
        }
    }

    fn nextImpl(self, pos: usize, in_unit: u16) -> CharTrieResult {
        let mut pos = pos;
        let node = self.chars_.chatAt(pos) as u16;
        pos += 1;

        loop {
            if node < MIN_LINEAR_MATCH {
                return self.branchNext(pos, node, in_unit);
            } else if node < MIN_VALUE_LEAD {
                // Match the first of length+1 units.
                let length = node - MIN_LINEAR_MATCH;
                pos += 1;
                if in_unit == self.chars_charAt(pos - 1) {
                    if length == 0 {
                        self.remainingMatchLength_ = None;
                        self.pos_ = Some(pos);
                        node = self.chars_.charAt(pos);
                        if node >= MIN_VALUE_LEAD {
                            return self.valueResults_[node >> 15];
                        }
                        return CharTrieResult::no_match();
                    } else {
                        self.remainingMatchLength_ = Some(length - 1);
                        self.pos_ = Some(pos);
                        return CharTrieResult::no_match();
                    }
                }
                // No match
                break;
            } else if (node & VALUE_IS_FINAL) != 0 {
                // No further matching units.
                break;
            } else {
                // Skip intermediate value.
                pos = self.skipNodeValue(pos, node);
                node &= NODE_TYPE_MASK;
            }
        }
        self.stop();
        CharTrieResult::no_match()
    }
}
