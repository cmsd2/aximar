cask "aximar" do
  arch arm: "aarch64", intel: "x64"

  version "0.15.1"
  sha256 arm:   "faefc98a5d8fa0e21b024772dad6ae3c22d32d307cf64b5419612e7f04a9a18f",
         intel: "b8d13404b5561a264f20ba29baaa6584e0c92a97b3e2a69cb79881cf6667c89e"

  url "https://github.com/cmsd2/aximar/releases/download/v#{version}/Aximar_#{version}_#{arch}.dmg"
  name "Aximar"
  desc "Desktop GUI for the Maxima computer algebra system"
  homepage "https://github.com/cmsd2/aximar"

  livecheck do
    url :url
    regex(/^v(\d+(?:\.\d+)+)$/i)
  end

  # Language server, debug adapter and MCP server, released separately.
  depends_on formula: "cmsd2/tap/aximar-tools"

  app "Aximar.app"

  zap trash: [
    "~/Library/Application Support/com.aximar.app",
    "~/Library/Caches/com.aximar.app",
    "~/Library/Preferences/com.aximar.app.plist",
    "~/Library/Saved Application State/com.aximar.app.savedState",
    "~/Library/WebKit/com.aximar.app",
  ]
end
