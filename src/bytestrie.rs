// 00..0f: Branch node. If node!=0 then the length is node+1, otherwise
// the length is one more than the next byte.

// For a branch sub-node with at most this many entries, we drop down
// to a linear search.
const MAX_BRANCH_LINEAR_SUB_NODE_LENGTH: usize = 5;

// 10..1f: Linear-match node, match 1..16 units and continue reading the next node.
const MIN_LINEAR_MATCH: u8 = 0x10;
const MAX_LINEAR_MATCH_LENGTH: u8 = 0x10;

const MIN_VALUE_LEAD: u8 = 0x20;

// A final-value node has bit 0 set.
const VALUE_IS_FINAL: u8 = 1;

// Compact value: After testing bit 0, shift right by 1 and then use the following thresholds.
const MIN_ONE_BYTE_VALUE_LEAD: u8 = MIN_VALUE_LEAD / 2; // 0x10
const MAX_ONE_BYTE_VALUE: u8 = 0x40; // At least 6 bits in the first byte.

const MIN_TWO_BYTE_VALUE_LEAD: u8 = MIN_ONE_BYTE_VALUE_LEAD + MAX_ONE_BYTE_VALUE + 1; // 0x51
const MAX_TWO_BYTE_VALUE: u32 = 0x1aff;
const MIN_THREE_BYTE_VALUE_LEAD: u8 =
    (MIN_TWO_BYTE_VALUE_LEAD + (MAX_TWO_BYTE_VALUE >> 8) as u8) + 1; // 0x6c
const FOUR_BYTE_VALUE_LEAD: u8 = 0x7e;

// A little more than Unicode code points. (0x11ffff)
const MAX_THREE_BYTE_VALUE: u32 =
    (((FOUR_BYTE_VALUE_LEAD - MIN_THREE_BYTE_VALUE_LEAD) as u32) << 16) - 1;

const FIVE_BYTE_VALUE_LEAD: u8 = 0x7f;

// A little more than Unicode code points. (0x11ffff)
/*
/*package*/ static final int kMinThreeByteValueLead=kMinTwoByteValueLead+(kMaxTwoByteValue>>8)+1;  // 0x6c
/*package*/ static final int kFourByteValueLead=0x7e;
*/

// Compact delta integers.
const MAX_ONE_BYTE_DELTA: u8 = 0xbf;
const MIN_TWO_BYTE_DELTA_LEAD: u8 = MAX_ONE_BYTE_DELTA + 1; // 0xc0
const MIN_THREE_BYTE_DELTA_LEAD: u8 = 0xf0;
const FOUR_BYTE_DELTA_LEAD: u8 = 0xfe;
const FIVE_BYTE_DELTA_LEAD: u8 = 0xff;
const MAX_TWO_BYTE_DELTA: u32 =
    (((MIN_THREE_BYTE_DELTA_LEAD - MIN_TWO_BYTE_DELTA_LEAD) as u32) << 8) - 1; // 0x2fff
const MAX_THREE_BYTE_DELTA: u32 =
    (((FOUR_BYTE_DELTA_LEAD - MIN_THREE_BYTE_DELTA_LEAD) as u32) << 16) - 1; // 0xdffff

fn skip_value(pos: usize, lead_byte: u8) -> usize {
    if lead_byte >= (MIN_TWO_BYTE_VALUE_LEAD << 1) {
        if lead_byte < (MIN_THREE_BYTE_VALUE_LEAD << 1) {
            pos + 1
        } else if lead_byte < (FOUR_BYTE_VALUE_LEAD << 1) {
            pos + 2
        } else {
            pos + 3 + ((lead_byte as usize >> 1) & 1)
        }
    } else {
        pos
    }
}

fn skip_delta(pos: usize, delta: u8) -> usize {
    if delta >= MIN_TWO_BYTE_DELTA_LEAD {
        if delta < MIN_THREE_BYTE_DELTA_LEAD {
            pos + 1
        } else if delta < FOUR_BYTE_DELTA_LEAD {
            pos + 2
        } else {
            pos + 3 + (delta as usize & 1)
        }
    } else {
        pos
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum BytesTrieState {
    NoValue,
    NoMatch,
    INTERMEDIATE,
    FinalValue,
}

#[derive(Clone, Copy)]
struct BytesTrieResult {
    state: BytesTrieState,
}

impl BytesTrieResult {
    pub fn no_value() -> Self {
        Self {
            state: BytesTrieState::NoValue,
        }
    }

    pub fn no_match() -> Self {
        Self {
            state: BytesTrieState::NoMatch,
        }
    }

    pub fn intermediate_value() -> Self {
        Self {
            state: BytesTrieState::INTERMEDIATE,
        }
    }

    pub fn final_value() -> Self {
        Self {
            state: BytesTrieState::FinalValue,
        }
    }

    pub fn is_final_value(self) -> bool {
        self.state == BytesTrieState::FinalValue
    }

    pub fn is_no_match(self) -> bool {
        self.state == BytesTrieState::NoMatch
    }
}

struct BytesTrie {
    bytes_: Vec<u8>,
    pos_: Option<usize>,
    root_: usize,
    remaining_match_length_: Option<usize>,
}

impl BytesTrie {
    pub fn new(trie: &[u8], offset: usize) -> Self {
        Self {
            bytes_: Vec::new(),
            pos_: Some(offset),
            root_: offset,
            remaining_match_length_: None,
        }
    }

    pub fn reset(&mut self) {
        self.pos_ = Some(self.root_);
        self.remaining_match_length_ = None;
    }

    pub fn current(&self) -> BytesTrieResult {
        if self.pos_.is_none() {
            return BytesTrieResult::no_match();
        }

        let pos = self.pos_.unwrap();
        let node = self.bytes_[pos];
        if self.remaining_match_length_.is_none() && node >= MIN_VALUE_LEAD {
            return BytesTrie::value_result(node);
        }

        BytesTrieResult::no_value()
    }

    // Traverses the trie from the initial state for this input char.
    // Equivalent to reset() then next(inUnit)
    pub fn first(&mut self, in_unit: u8) -> BytesTrieResult {
        self.remaining_match_length_ = None;
        self.next_impl(self.root_, in_unit)
    }

    // Traverses the trie from the current state for this input char.
    pub fn next_in_unit(&mut self, in_unit: u8) -> BytesTrieResult {
        if self.pos_.is_none() {
            return BytesTrieResult::no_match();
        }

        let mut pos = self.pos_.unwrap();
        if let Some(length) = self.remaining_match_length_ {
            // Remaining part of a linear-match node
            if in_unit == self.bytes_[pos] {
                pos += 1;
                self.pos_ = Some(pos);
                if length == 0 {
                    self.remaining_match_length_ = None;
                    let node = self.bytes_[pos];
                    if node >= MIN_VALUE_LEAD {
                        return BytesTrie::value_result(node);
                    }
                    return BytesTrieResult::no_value();
                } else {
                    self.remaining_match_length_ = Some(length);
                    return BytesTrieResult::no_value();
                }
            }
            self.stop();
            // no match
            return BytesTrieResult::no_match();
        }
        self.next_impl(pos, in_unit)
    }

    // Traverses the trie from the current state for this string.
    // Equivalent to
    pub fn next(&mut self, s: &[u8], index: usize, limit: usize) -> BytesTrieResult {
        if index >= limit {
            // Empty index.
            return self.current();
        }
        if self.pos_.is_none() {
            return BytesTrieResult::no_match();
        }

        let mut index = index;
        let mut pos = self.pos_.unwrap();
        let mut length = self.remaining_match_length_;

        loop {
            let mut in_unit;
            loop {
                if index == limit {
                    self.remaining_match_length_ = length;
                    self.pos_ = Some(pos);
                    if length.is_none() {
                        let node = self.bytes_[pos];
                        if node >= MIN_VALUE_LEAD {
                            return BytesTrie::value_result(node);
                        }
                    }
                    // no value
                    return BytesTrieResult::no_value();
                }
                in_unit = s[index];
                index += 1;
                if length.is_none() {
                    self.remaining_match_length_ = None;
                    break;
                }

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
            }
            loop {
                let node = self.bytes_[pos];
                pos += 1;
                if node < MIN_LINEAR_MATCH {
                    let result = self.branch_next(pos, node as usize, in_unit);
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
                    pos = self.pos_.unwrap(); // branch_next() advanced pos and wrote it to self.pos_;
                } else if node < MIN_VALUE_LEAD {
                    length = Some((node - MIN_LINEAR_MATCH) as usize);
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
                    pos = skip_value(pos, node);
                }
            }
        }
    }

    fn branch_next(&mut self, pos: usize, length: usize, in_unit: u8) -> BytesTrieResult {
        let mut pos = pos;
        let mut length = length;
        if length == 0 {
            length = self.bytes_[pos] as usize;
            pos += 1;
        }
        length += 1;

        // The length of the branch is the number of units to select from.
        // The data structure encodes a binary search.
        while length > MAX_BRANCH_LINEAR_SUB_NODE_LENGTH {
            if in_unit < self.bytes_[pos] {
                length >>= 1;
                pos = self.jump_by_delta(pos + 1);
            } else {
                length = length - (length >> 1);
                pos = self.skip_delta(pos + 1);
            }
        }
        // Drop down to linear search for the last few bytes.
        // length>=2 because the loop body above sees length>kMaxBranchLinearSubNodeLength>=3
        // and divides length by 2.
        loop {
            if in_unit == self.bytes_[pos] {
                pos += 1;
                let mut node = self.bytes_[pos];
                if node & VALUE_IS_FINAL != 0 {
                    // Leave the final value for getValue() to read.
                    self.pos_ = Some(pos);
                    return BytesTrieResult::final_value();
                }
                // Use the non-final value as the jump delta.
                pos += 1;

                node >>= 1;
                let delta;
                if node < MIN_TWO_BYTE_VALUE_LEAD {
                    delta = (node - MIN_ONE_BYTE_VALUE_LEAD) as usize;
                } else if node < MIN_THREE_BYTE_VALUE_LEAD {
                    delta = ((node - MIN_TWO_BYTE_VALUE_LEAD) << 8) as usize
                        | self.bytes_[pos] as usize;
                    pos += 1;
                } else if node < FOUR_BYTE_VALUE_LEAD {
                    delta = (((node - MIN_THREE_BYTE_VALUE_LEAD) as usize) << 16)
                        | (self.bytes_[pos] as usize) << 8
                        | self.bytes_[pos + 1] as usize;
                    pos += 2;
                } else if node == FOUR_BYTE_VALUE_LEAD {
                    delta = (self.bytes_[pos] as usize) << 16
                        | (self.bytes_[pos + 1] as usize) << 8
                        | self.bytes_[pos + 2] as usize;
                } else {
                    delta = (self.bytes_[pos] as usize) << 24
                        | (self.bytes_[pos + 1] as usize) << 16
                        | (self.bytes_[pos + 2] as usize) << 8
                        | self.bytes_[pos + 3] as usize;
                }
                pos = pos + delta;
                node = self.bytes_[pos];
                if node >= MIN_VALUE_LEAD {
                    return BytesTrie::value_result(node);
                }
                return BytesTrieResult::no_value();
            } else {
                pos += 1;
            }
            length -= 1;
            pos = self.skip_value(pos);
            if length > 1 {
                continue;
            }
            break;
        }

        if in_unit == self.bytes_[pos] {
            self.pos_ = Some(pos + 1);
            let node = self.bytes_[pos + 1];
            if node >= MIN_VALUE_LEAD {
                return BytesTrie::value_result(node);
            }
            return BytesTrieResult::no_value();
        }
        self.stop();
        return BytesTrieResult::no_match();
    }

    fn next_impl(&mut self, pos: usize, in_unit: u8) -> BytesTrieResult {
        let mut pos = pos;
        loop {
            let mut node = self.bytes_[pos];
            if node < MIN_LINEAR_MATCH {
                return self.branch_next(pos, node as usize, in_unit);
            } else if node < MIN_VALUE_LEAD {
                // Match the first of length+1 units.
                let length = node - MIN_LINEAR_MATCH;
                if in_unit == self.bytes_[pos] {
                    if length == 0 {
                        pos += 1;
                        self.remaining_match_length_ = None;
                        self.pos_ = Some(pos);
                        node = self.bytes_[pos];
                        if node >= MIN_VALUE_LEAD {
                            return BytesTrie::value_result(node);
                        }
                        return BytesTrieResult::no_value();
                    }
                    self.remaining_match_length_ = Some(length as usize - 1);
                    self.pos_ = Some(pos + 1);
                    return BytesTrieResult::no_value();
                }
                // No match
                break;
            } else if (node & VALUE_IS_FINAL) != 0 {
                // No further matching units.
                break;
            } else {
                // Skip intermediate value.
                pos = skip_value(pos, node);
            }
        }
        self.stop();
        BytesTrieResult::no_match()
    }

    fn stop(&mut self) {
        self.pos_ = None;
    }

    fn jump_by_delta(&self, pos: usize) -> usize {
        let delta = self.bytes_[pos];
        if delta < MIN_TWO_BYTE_DELTA_LEAD {
            // nothing to do
            pos + 1 + delta as usize
        } else if delta < MIN_THREE_BYTE_DELTA_LEAD {
            let delta =
                (((delta - MIN_TWO_BYTE_DELTA_LEAD) as usize) << 8) | self.bytes_[pos + 1] as usize;
            pos + delta + 2
        } else if delta < FOUR_BYTE_DELTA_LEAD {
            let delta = (((delta - MIN_THREE_BYTE_DELTA_LEAD) as usize) << 16)
                | ((self.bytes_[pos + 1] as usize) << 8)
                | self.bytes_[pos + 2] as usize;
            pos + delta + 3
        } else if delta == FOUR_BYTE_DELTA_LEAD {
            let delta = ((self.bytes_[pos + 1] as usize) << 16)
                | ((self.bytes_[pos + 2] as usize) << 8)
                | (self.bytes_[pos + 3] as usize);
            pos + delta + 4
        } else {
            let delta = ((self.bytes_[pos + 1] as usize) << 24)
                | ((self.bytes_[pos + 2] as usize) << 16)
                | ((self.bytes_[pos + 3] as usize) << 8)
                | (self.bytes_[pos + 4] as usize);
            pos + delta + 5
        }
    }

    fn skip_value(&self, pos: usize) -> usize {
        let lead_byte = self.bytes_[pos];
        skip_value(pos + 1, lead_byte)
    }

    fn skip_delta(&self, pos: usize) -> usize {
        let lead_byte = self.bytes_[pos];
        skip_delta(pos + 1, lead_byte)
    }

    fn value_result(node: u8) -> BytesTrieResult {
        let node = node & VALUE_IS_FINAL;
        match node {
            VALUE_IS_FINAL => BytesTrieResult::final_value(),
            _ => BytesTrieResult::intermediate_value(),
        }
    }
}
