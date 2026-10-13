class Omega < Formula
  desc "High-performance terminal session and workspace navigator"
  homepage "https://github.com/kiyors/omega"
  url "https://github.com/kiyors/omega/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  license "MIT"
  head "https://github.com/kiyors/omega.git", branch: "main"

  depends_on "rust" => :build
  depends_on "tmux" => :recommended

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "omega", shell_output("#{bin}/omega --version")
    assert_match "Omega Health Diagnostics", shell_output("#{bin}/omega doctor")
  end
end
