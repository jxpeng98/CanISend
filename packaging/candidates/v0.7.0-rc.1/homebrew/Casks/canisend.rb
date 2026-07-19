cask "canisend" do
  arch arm: "aarch64", intel: "x86_64"

  version "0.7.0-rc.1"
  sha256 arm:   "acb8f0d1f6f4b14881e13c717326cdf80681c5b3631ff93ecd77a6fad7c4f35e",
         intel: "4a36e95e103272485b977e6684b4ed4ef482aee67242f6b9932fa2acf77c74ea"

  url "https://github.com/jxpeng98/CanISend/releases/download/v#{version}/canisend-#{version}-#{arch}-apple-darwin.tar.gz"
  name "CanISend"
  desc "Prepare evidence-backed academic job applications with agent hosts"
  homepage "https://github.com/jxpeng98/CanISend"

  binary "canisend-#{version}-#{arch}-apple-darwin/canisend"
end
