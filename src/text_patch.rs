#![allow(dead_code)]

pub(crate) const PAGE: usize = 0x1000;

pub(crate) const CHECK_WINDOW: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Chunk {
    pub start: usize,
    pub lead: usize,
    pub len: usize,
}

pub(crate) fn plan_chunks(address: usize, len: usize) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut cursor = address;
    let mut done = 0usize;
    while done < len {
        let page_end = (cursor & !(PAGE - 1)) + PAGE;
        let take = core::cmp::min(len - done, page_end - cursor);
        let lead = (cursor & (PAGE - 1)).saturating_sub(PAGE - CHECK_WINDOW);
        chunks.push(Chunk {
            start: cursor - lead,
            lead,
            len: take,
        });
        cursor += take;
        done += take;
    }
    chunks
}

pub(crate) unsafe fn write_bytes(address: usize, bytes: &[u8]) -> bool {
    let mut done = 0usize;
    for chunk in plan_chunks(address, bytes.len()) {
        let mut buffer = Vec::with_capacity(chunk.lead + chunk.len);
        buffer.extend_from_slice(core::slice::from_raw_parts(
            chunk.start as *const u8,
            chunk.lead,
        ));
        buffer.extend_from_slice(&bytes[done..done + chunk.len]);
        let result =
            skyline::patching::sky_memcpy(chunk.start as _, buffer.as_ptr() as _, buffer.len());
        if result.0.is_some() {
            return false;
        }
        done += chunk.len;
    }
    true
}

pub(crate) unsafe fn write_words(address: usize, words: &[u32]) -> bool {
    write_bytes(
        address,
        core::slice::from_raw_parts(words.as_ptr() as *const u8, words.len() * 4),
    )
}

pub(crate) unsafe fn write_word(address: usize, word: u32) -> bool {
    write_words(address, &[word])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fits(chunk: &Chunk) -> bool {
        let offset = chunk.start & (PAGE - 1);
        offset + core::cmp::max(chunk.lead + chunk.len, CHECK_WINDOW) <= PAGE
    }

    #[test]
    fn a_mid_page_pair_is_one_untouched_chunk() {
        let chunks = plan_chunks(0x25f7064, 8);
        assert_eq!(
            chunks,
            vec![Chunk {
                start: 0x25f7064,
                lead: 0,
                len: 8
            }]
        );
        assert!(chunks.iter().all(fits));
    }

    #[test]
    fn the_last_fitting_window_is_left_alone() {
        let chunks = plan_chunks(0x1c2aff8, 8);
        assert_eq!(chunks[0].lead, 0);
        assert_eq!(chunks.len(), 1);
        assert!(chunks.iter().all(fits));
    }

    #[test]
    fn a_pair_that_straddles_a_page_splits_and_shifts_down() {
        let chunks = plan_chunks(0x25f7ffc, 8);
        assert_eq!(
            chunks,
            vec![
                Chunk {
                    start: 0x25f7ff8,
                    lead: 4,
                    len: 4
                },
                Chunk {
                    start: 0x25f8000,
                    lead: 0,
                    len: 4
                }
            ]
        );
        assert!(chunks.iter().all(fits));
    }

    #[test]
    fn a_single_word_at_the_page_tail_shifts_down() {
        let chunks = plan_chunks(0x18d3ffc, 4);
        assert_eq!(
            chunks,
            vec![Chunk {
                start: 0x18d3ff8,
                lead: 4,
                len: 4
            }]
        );
        assert!(chunks.iter().all(fits));
    }

    #[test]
    fn every_word_aligned_address_and_length_stays_inside_its_page() {
        for offset in (0..PAGE).step_by(4) {
            for len in [4usize, 8, 12, 16, 64] {
                let chunks = plan_chunks(0x2000_0000 + offset, len);
                assert_eq!(chunks.iter().map(|chunk| chunk.len).sum::<usize>(), len);
                for chunk in &chunks {
                    assert!(fits(chunk), "offset {offset:#x} len {len} chunk {chunk:?}");
                }
            }
        }
    }

    #[test]
    fn chunks_are_contiguous_and_ascending() {
        let chunks = plan_chunks(0x1000_0ff8, 32);
        let mut expected = 0x1000_0ff8usize;
        for chunk in &chunks {
            assert_eq!(chunk.start + chunk.lead, expected);
            expected += chunk.len;
        }
        assert_eq!(expected, 0x1000_0ff8 + 32);
    }
}
