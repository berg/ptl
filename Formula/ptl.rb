# This file is a template — placeholders are substituted by the release workflow.
# The rendered version lives in https://github.com/berg/homebrew-ptl
class Ptl < Formula
  desc "Brother P-Touch label printer CLI"
  homepage "https://github.com/berg/ptl"
  version "__VERSION__"
  license "GPL-3.0"

  on_macos do
    on_intel do
      url "https://github.com/berg/ptl/releases/download/v__VERSION__/ptl-v__VERSION__-x86_64-apple-darwin.tar.gz"
      sha256 "__SHA256_X86_MACOS__"
    end
    on_arm do
      url "https://github.com/berg/ptl/releases/download/v__VERSION__/ptl-v__VERSION__-aarch64-apple-darwin.tar.gz"
      sha256 "__SHA256_ARM_MACOS__"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/berg/ptl/releases/download/v__VERSION__/ptl-v__VERSION__-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "__SHA256_X86_LINUX__"
    end
    on_arm do
      url "https://github.com/berg/ptl/releases/download/v__VERSION__/ptl-v__VERSION__-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "__SHA256_ARM_LINUX__"
    end
  end

  depends_on "libusb"

  def install
    bin.install "ptl"
  end

  test do
    assert_match "ptl", shell_output("#{bin}/ptl --help 2>&1")
  end
end
