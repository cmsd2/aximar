class AximarTools < Formula
  desc "Maxima language server, debug adapter and MCP server from the Aximar project"
  homepage "https://github.com/cmsd2/aximar"
  version "0.5.0"
  license "GPL-3.0-or-later"

  on_macos do
    on_arm do
      url "https://github.com/cmsd2/aximar/releases/download/tools-v0.5.0/aximar-tools-aarch64-apple-darwin.tar.gz"
      sha256 "4144750f418fe2af9c76e1ff8bbe93e6c4e990e7e9d6df2a18a60fa936b21b3f"
    end
    on_intel do
      url "https://github.com/cmsd2/aximar/releases/download/tools-v0.5.0/aximar-tools-x86_64-apple-darwin.tar.gz"
      sha256 "7e303851bf3b4bbce6bc86fc0b42f10e3e6a80267cd7553ebc33f36213cc0acb"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/cmsd2/aximar/releases/download/tools-v0.5.0/aximar-tools-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "110001ef95435fdadcba54d516d2f182c39d228afe8f79a9ae93682c6710e2ce"
    end
    on_intel do
      url "https://github.com/cmsd2/aximar/releases/download/tools-v0.5.0/aximar-tools-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "9f8a04e7a91b3185aa6410026346680cc7e814e38dc734f2bd174efea6ebe88e"
    end
  end

  def install
    bin.install "maxima-lsp", "maxima-dap", "aximar-mcp"
  end

  test do
    assert_predicate bin/"maxima-lsp", :executable?
    assert_predicate bin/"maxima-dap", :executable?
    assert_predicate bin/"aximar-mcp", :executable?
  end
end
