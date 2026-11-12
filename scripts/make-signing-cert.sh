#!/usr/bin/env bash
# Creates a self-signed macOS code-signing certificate for the preview and prints the GitHub
# secrets that give every build the same code identity (007 T059). A stable identity keeps the
# Accessibility grant across updates; it does not satisfy Gatekeeper (not notarized), so the
# first-launch warning stays until T028. Run once on the Mac and keep the .p12 + password offline.
#
# Usage: scripts/make-signing-cert.sh [output-dir]
set -euo pipefail

out="${1:-$HOME/Desktop/verso-signing}"
name="Verso Self-Signed"
mkdir -p "$out"
chmod 700 "$out"

password=$(openssl rand -base64 32 | tr -dc 'A-Za-z0-9' | cut -c1-24)

openssl req -x509 -newkey rsa:2048 -nodes -days 3650 \
  -keyout "$out/key.pem" -out "$out/cert.pem" \
  -subj "/CN=${name}" \
  -addext "basicConstraints=critical,CA:false" \
  -addext "keyUsage=critical,digitalSignature" \
  -addext "extendedKeyUsage=critical,codeSigning"

openssl pkcs12 -export -legacy -inkey "$out/key.pem" -in "$out/cert.pem" \
  -out "$out/verso-signing.p12" -name "$name" -passout "pass:${password}"

# GitHub secrets hold one base64 string; `base64 -w0` is GNU, macOS needs -i and a newline strip.
base64 -i "$out/verso-signing.p12" | tr -d '\n' >"$out/verso-signing.p12.b64"
# The certificate itself is public; the CI trusts it for code signing before building.
cp "$out/cert.pem" "$out/verso-signing-cert.pem"
rm -f "$out/key.pem"
chmod 600 "$out/verso-signing.p12" "$out/verso-signing.p12.b64"

cat <<EOF
Certificate: $out/verso-signing.p12
Public cert: $out/verso-signing-cert.pem
Base64:      $out/verso-signing.p12.b64
Identity:    $name
Password:    $password

Save the .p12 and this password offline. Then set the repository secrets and variable:

  gh secret set APPLE_CERTIFICATE --repo lbngoc/verso < "$out/verso-signing.p12.b64"
  gh secret set APPLE_CERTIFICATE_PASSWORD --repo lbngoc/verso --body "$password"
  gh secret set APPLE_CERTIFICATE_PUBLIC --repo lbngoc/verso < "$out/verso-signing-cert.pem"
  gh variable set APPLE_SIGNING_IDENTITY --repo lbngoc/verso --body "$name"

The private key never goes into the repository, the app or the logs.
EOF
