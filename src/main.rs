use anyhow::Result;
use rand::seq::SliceRandom;
use reed_solomon_erasure::galois_8::ReedSolomon;

fn main() -> Result<()> {
    // MCP erasure split: q/4 data slices, rest parity, fanned out to
    // ~256 attesters. Any `data_shards` shreds reconstruct the whole thing.
    let rs = ReedSolomon::new(64, 192)?;
    let data_shards = rs.data_shard_count(); // slices of T needed to rebuild (q/4)
    let parity_shards = rs.parity_shard_count(); // redundancy
    let total = rs.total_shard_count(); // shreds sent

    // the proposer's transaction list T as actual bytes.
    let payload: &[u8] = b"tx1: alice->bob 5 SOL; tx2: carol->dave 1 SOL; tx3: bob->erin 2 SOL";
    println!(
        "sending T ({} bytes): {:?}\n",
        payload.len(),
        String::from_utf8_lossy(payload)
    );

    // PSLICES: cut T into `data_shards` equal-length slices. All shards must be
    // the same length, so we pick a shard_len big enough to hold the payload
    // across `data_shards` slices, and zero-pad the tail.
    let shard_len = payload.len().div_ceil(data_shards); // bytes per slice
    let padded_len = shard_len * data_shards;
    let mut buf = payload.to_vec();
    buf.resize(padded_len, 0); // zero-pad so every slice is full

    let pslices: Vec<Vec<u8>> = buf
        .chunks(shard_len) // exactly `data_shards` chunks of `shard_len`
        .map(|c| c.to_vec())
        .collect();

    // PSHREDS: erasure-code the slices. encode() needs every buffer present, so
    // start from the data slices, append empty parity buffers, then encode()
    // fills the parity. This is what the proposer fans out to the attesters.
    let mut pshreds: Vec<Vec<u8>> = pslices.clone();
    pshreds.extend((0..parity_shards).map(|_| vec![0u8; shard_len]));
    rs.encode(&mut pshreds)?;

    // RECEIVED: an attester only sees a subset. None = that shred never arrived.
    // Drop the max we can (the parity count), leaving exactly `data_shards`.
    let mut received: Vec<Option<Vec<u8>>> = pshreds.into_iter().map(Some).collect();

    // Lose `parity_shards` shreds at RANDOM, scattered positions (like real
    // network loss) instead of a contiguous block. Shuffle all indices and drop
    // the first `parity_shards` of them, so exactly `data_shards` survive.
    let mut idx: Vec<usize> = (0..total).collect();
    idx.shuffle(&mut rand::thread_rng());

    for &i in idx.iter().take(parity_shards) {
        received[i] = None;
    }
    let held = received.iter().filter(|s| s.is_some()).count();
    println!("fanned out {total} pshreds; attester holds {held} (need >= {data_shards})");

    // RECONSTRUCT: rebuild the full pshred set from whatever arrived.
    rs.reconstruct(&mut received)?;
    let recovered: Vec<Vec<u8>> = received.into_iter().map(|s| s.unwrap()).collect();

    // REASSEMBLE T: concatenate the first `data_shards` recovered slices and
    // trim the zero padding back off to get the exact original bytes.
    let mut bytes: Vec<u8> = recovered[..data_shards].concat();
    bytes.truncate(payload.len());

    // Print the actual reconstructed data and compare it to what we sent.
    println!("sent      T : {:?}", String::from_utf8_lossy(payload));
    println!("recovered T : {:?}", String::from_utf8_lossy(&bytes));

    let same = bytes == payload;
    println!("data matches original? {same}");
    println!("rebuilt the transaction list from only {data_shards} of {total} shreds");

    Ok(())
}
