#include <algorithm>
#include <memory>
#include <sweeplsd/sweeplsd.hpp>

// Rust validates the image dimensions and buffer. Exceptions stay inside C++.
extern "C" bool sweeplsd_detect(const unsigned char *pixels, int width, int height,
                                float **output, std::size_t *count) noexcept {
    try {
        sweeplsd::GrayImage image(width, height);
        std::copy_n(pixels, image.data.size(), image.data.begin());
        const auto segments = sweeplsd::detectOnePass(image);
        auto lines = std::make_unique<float[]>(segments.size() * 4);
        for (std::size_t i = 0; i < segments.size(); ++i) {
            const auto &line = segments[i];
            lines[i * 4] = line.x0;
            lines[i * 4 + 1] = line.y0;
            lines[i * 4 + 2] = line.x1;
            lines[i * 4 + 3] = line.y1;
        }
        *count = segments.size();
        *output = lines.release();
        return true;
    } catch (...) {
        return false;
    }
}

extern "C" void sweeplsd_free(float *lines) noexcept {
    delete[] lines;
}
