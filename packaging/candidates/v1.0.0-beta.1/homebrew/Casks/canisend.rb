cask "canisend" do
  arch arm: "aarch64", intel: "x86_64"

  version "1.0.0-beta.1"
  sha256 arm:   "52d982f8a5a8cc9a2eb564df5994b42e0825734ef086e3da432846acb2352522",
         intel: "246580fac393fab2b7a00a9b7358c455bf9136ba24d0ba29abb1e55b7b196243"

  url "https://github.com/jxpeng98/CanISend/releases/download/v#{version}/canisend-#{version}-#{arch}-apple-darwin.tar.gz"
  name "CanISend"
  desc "Prepare evidence-bound applications and submissions locally"
  homepage "https://github.com/jxpeng98/CanISend"

  binary "canisend-#{version}-#{arch}-apple-darwin/canisend"
end
