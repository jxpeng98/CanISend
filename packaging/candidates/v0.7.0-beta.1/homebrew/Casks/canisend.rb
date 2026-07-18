cask "canisend" do
  arch arm: "aarch64", intel: "x86_64"

  version "0.7.0-beta.1"
  sha256 arm:   "f485a98bce61ab2ad929d566e10b4f196675890bc7aed1845d0888dd53a0543d",
         intel: "5994bf09c2f8c8fefc4e0fe228a4f70feaa2ff55e72185fdabe786249e784326"

  url "https://github.com/jxpeng98/CanISend/releases/download/v#{version}/canisend-#{version}-#{arch}-apple-darwin.tar.gz"
  name "CanISend"
  desc "Prepare evidence-backed academic job applications with agent hosts"
  homepage "https://github.com/jxpeng98/CanISend"

  binary "canisend-#{version}-#{arch}-apple-darwin/canisend"
end
