unsafe extern "C" {
    fn sweeplsd_detect(
        pixels: *const u8,
        width: i32,
        height: i32,
        output: *mut *mut f32,
        count: *mut usize,
    ) -> bool;
    fn sweeplsd_free(lines: *mut f32);
}

/// Detect line segments in a row-major, 8-bit grayscale image.
/// Each segment is `[x0, y0, x1, y1]` in pixel coordinates.
pub fn detect(pixels: &[u8], width: usize, height: usize) -> anyhow::Result<Vec<[f32; 4]>> {
    let len = width
        .checked_mul(height)
        .ok_or_else(|| anyhow::anyhow!("Image size overflow"))?;
    anyhow::ensure!(
        width <= (i32::MAX - 8) as usize
            && height <= (i32::MAX - 8) as usize
            && len <= i32::MAX as usize
            && pixels.len() == len,
        "Invalid grayscale image dimensions"
    );
    // The detector excludes a 3px border on each side.
    if width <= 6 || height <= 6 {
        return Ok(Vec::new());
    }
    let mut output = std::ptr::null_mut();
    let mut count = 0;
    // SAFETY: The validated buffer contains width * height pixels.
    let success = unsafe {
        sweeplsd_detect(
            pixels.as_ptr(),
            width as i32,
            height as i32,
            &mut output,
            &mut count,
        )
    };
    anyhow::ensure!(success, "SweepLSD detection failed");
    let result = (|| {
        let mut lines = Vec::new();
        lines.try_reserve_exact(count)?;
        // SAFETY: A successful call returns count * 4 allocated floats.
        for line in unsafe { std::slice::from_raw_parts(output, count * 4) }.chunks_exact(4) {
            lines.push([line[0], line[1], line[2], line[3]]);
        }
        anyhow::Ok(lines)
    })();
    // SAFETY: Free the C++ allocation exactly once, including on reserve failure.
    unsafe { sweeplsd_free(output) };
    result
}
