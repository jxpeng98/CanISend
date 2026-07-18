cask "canisend" do
  arch arm: "aarch64", intel: "x86_64"

  version "0.7.0-alpha.1"
  sha256 arm:   "7668a7c48878f755e6ac43abddfc50e2d10c4ed495ab94c3e221d98470eec572",
         intel: "4a50c1f2d58ea657116fad05463e675bdf67bc5df04f153eafefd648a6c335d3"

  url "https://github.com/jxpeng98/CanISend/releases/download/v#{version}/canisend-#{version}-#{arch}-apple-darwin.tar.gz"
  name "CanISend"
  desc "Prepare evidence-backed academic job applications with agent hosts"
  homepage "https://github.com/jxpeng98/CanISend"

  binary "canisend-#{version}-#{arch}-apple-darwin/canisend"
end
