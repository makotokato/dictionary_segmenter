use crate::trie::*;

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
    assert!(lead_byte >= MIN_VALUE_LEAD);
    if lead_byte < (MIN_TWO_BYTE_VALUE_LEAD << 1) {
        pos
    } else if lead_byte < (MIN_THREE_BYTE_VALUE_LEAD << 1) {
        pos + 1
    } else if lead_byte < (FOUR_BYTE_VALUE_LEAD << 1) {
        pos + 2
    } else {
        pos + 3 + ((lead_byte >> 1) & 1) as usize
    }
}

#[derive(Clone)]
pub struct BytesTrie<'a> {
    bytes_: &'a [u8],
    pos_: Option<usize>,
    root_: usize,
    remaining_match_length_: Option<usize>,
}

impl<'a> Trie for BytesTrie<'a> {
    // Traverses the trie from the initial state for this input char.
    // Equivalent to reset() then next(inUnit)
    fn first(&mut self, in_byte: i32) -> TrieResult {
        let mut in_byte = in_byte;
        self.remaining_match_length_ = None;
        if in_byte < 0 {
            in_byte += 0x100;
        }
        self.next_impl(self.root_, in_byte as u8)
    }

    // Traverses the trie from the current state for this input char.
    fn next(&mut self, in_byte: i32) -> TrieResult {
        let mut in_byte = in_byte;
        if self.pos_.is_none() {
            return TrieResult::NoMatch;
        }
        if in_byte < 0 {
            in_byte += 0x100;
        }
        let in_byte = in_byte as u8;
        let mut pos = self.pos_.unwrap();
        if let Some(length) = self.remaining_match_length_ {
            // Remaining part of a linear-match node
            if in_byte == self.bytes_[pos] {
                pos += 1;
                self.pos_ = Some(pos);
                if length == 0 {
                    self.remaining_match_length_ = None;
                    let node = self.bytes_[pos];
                    if node >= MIN_VALUE_LEAD {
                        return BytesTrie::value_result(node);
                    }
                } else {
                    self.remaining_match_length_ = Some(length);
                }
                return TrieResult::NoValue;
            }
            self.stop();
            TrieResult::NoMatch
        } else {
            self.next_impl(pos, in_byte)
        }
    }
}

impl<'a> BytesTrie<'a> {
    pub fn new(trie: &'a [u8], offset: usize) -> Self {
        Self {
            bytes_: trie,
            pos_: Some(offset),
            root_: offset,
            remaining_match_length_: None,
        }
    }

    fn branch_next(&mut self, pos: usize, length: usize, in_unit: u8) -> TrieResult {
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
                assert!(node >= MIN_VALUE_LEAD);
                if node & VALUE_IS_FINAL != 0 {
                    // Leave the final value for getValue() to read.
                    self.pos_ = Some(pos);
                    return TrieResult::FinalValue;
                }
                // Use the non-final value as the jump delta.
                pos += 1;

                node >>= 1;
                if node < MIN_TWO_BYTE_VALUE_LEAD {
                    pos += (node - MIN_ONE_BYTE_VALUE_LEAD) as usize;
                } else if node < MIN_THREE_BYTE_VALUE_LEAD {
                    pos += (((node - MIN_TWO_BYTE_VALUE_LEAD) as u32) << 8) as usize
                        | self.bytes_[pos] as usize;
                    pos += 1;
                } else if node < FOUR_BYTE_VALUE_LEAD {
                    pos += (((node - MIN_THREE_BYTE_VALUE_LEAD) as usize) << 16)
                        | (self.bytes_[pos] as usize) << 8
                        | self.bytes_[pos + 1] as usize;
                    pos += 2;
                } else if node == FOUR_BYTE_VALUE_LEAD {
                    pos += (self.bytes_[pos] as usize) << 16
                        | (self.bytes_[pos + 1] as usize) << 8
                        | self.bytes_[pos + 2] as usize;
                    pos += 3;
                } else {
                    pos += (self.bytes_[pos] as usize) << 24
                        | (self.bytes_[pos + 1] as usize) << 16
                        | (self.bytes_[pos + 2] as usize) << 8
                        | self.bytes_[pos + 3] as usize;
                    pos += 4;
                }
                node = self.bytes_[pos];
                self.pos_ = Some(pos);

                if node >= MIN_VALUE_LEAD {
                    return BytesTrie::value_result(node);
                }
                return TrieResult::NoValue;
            }
            length -= 1;
            pos = self.skip_value(pos + 1);
            if length <= 1 {
                break;
            }
        }

        if in_unit == self.bytes_[pos] {
            pos += 1;
            self.pos_ = Some(pos);
            let node = self.bytes_[pos];
            if node >= MIN_VALUE_LEAD {
                return BytesTrie::value_result(node);
            }
            TrieResult::NoValue
        } else {
            self.stop();
            TrieResult::NoMatch
        }
    }

    fn next_impl(&mut self, pos: usize, in_unit: u8) -> TrieResult {
        let mut pos = pos;
        loop {
            let mut node = self.bytes_[pos];
            pos += 1;
            if node < MIN_LINEAR_MATCH {
                return self.branch_next(pos, node as usize, in_unit);
            } else if node < MIN_VALUE_LEAD {
                // Match the first of length+1 units.
                let length = node - MIN_LINEAR_MATCH;
                if in_unit == self.bytes_[pos] {
                    pos += 1;
                    if length == 0 {
                        self.remaining_match_length_ = None;
                        self.pos_ = Some(pos);
                        node = self.bytes_[pos];
                        if node >= MIN_VALUE_LEAD {
                            return BytesTrie::value_result(node);
                        }
                        return TrieResult::NoValue;
                    }
                    self.remaining_match_length_ = Some(length as usize - 1);
                    self.pos_ = Some(pos);
                    return TrieResult::NoValue;
                }
                // No match
                break;
            } else if (node & VALUE_IS_FINAL) != 0 {
                // No further matching units.
                break;
            } else {
                // Skip intermediate value.
                pos = skip_value(pos, node);
                assert!(self.bytes_[pos] < MIN_VALUE_LEAD);
            }
        }
        self.stop();
        TrieResult::NoMatch
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
        let delta = self.bytes_[pos];
        if delta < MIN_TWO_BYTE_DELTA_LEAD {
            pos + 1
        } else if delta < MIN_THREE_BYTE_DELTA_LEAD {
            pos + 2
        } else if delta < FOUR_BYTE_DELTA_LEAD {
            pos + 3
        } else {
            pos + 4 + ((delta & 1) as usize)
        }
    }

    fn value_result(node: u8) -> TrieResult {
        let node = node & VALUE_IS_FINAL;
        match node {
            VALUE_IS_FINAL => TrieResult::FinalValue,
            _ => TrieResult::Intermediate,
        }
    }
}
