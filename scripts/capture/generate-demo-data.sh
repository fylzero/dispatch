#!/bin/bash
# Builds the throwaway HOME that capture.sh runs Dispatch in: a small .zshrc and
# an invented Node project with git history, so the prompt shows its Node,
# folder, branch and changes chips. Every name and address here is made up.
#
# Usage: generate-demo-data.sh <home>
set -euo pipefail

DEMO_HOME="${1:?usage: generate-demo-data.sh <home>}"
PROJECT="$DEMO_HOME/Projects/acme-web"

# Never pick up the real user's git identity or config.
export GIT_CONFIG_GLOBAL=/dev/null
export GIT_CONFIG_SYSTEM=/dev/null

mkdir -p "$DEMO_HOME" "$PROJECT"

# ------------------------------------------------------------- the shell ----
# A real node binary, not a version manager's shim that needs the real HOME.
NODE_BIN=""
for candidate in /opt/homebrew/bin/node /usr/local/bin/node "$(command -v node 2>/dev/null)"; do
  [[ -x "$candidate" ]] && HOME="$DEMO_HOME" "$candidate" --version >/dev/null 2>&1 && { NODE_BIN="$candidate"; break; }
done
[[ -n "$NODE_BIN" ]] || { echo "No working node found (install one with: brew install node)" >&2; exit 1; }
NODE_DIR="$(dirname "$NODE_BIN")"
cat > "$DEMO_HOME/.zshrc" <<ZSH
export PATH="$NODE_DIR:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"
export GIT_CONFIG_GLOBAL="\$HOME/.gitconfig"
export GIT_CONFIG_SYSTEM=/dev/null
export CLICOLOR=1
export LSCOLORS=ExGxFxdaCxDaDahbadacec
export NO_UPDATE_NOTIFIER=1
export PAGER=cat GIT_PAGER=cat
HISTFILE="\$HOME/.zsh_history"
alias ll='ls -lh'
ZSH
cat > "$DEMO_HOME/.gitconfig" <<'GIT'
[user]
  name = Alex Rivers
  email = alex@example.local
[init]
  defaultBranch = main
[core]
  pager = cat
[color]
  ui = auto
[advice]
  detachedHead = false
GIT
: > "$DEMO_HOME/.zsh_history"
# No "Last login" banner or macOS zsh session restore noise.
touch "$DEMO_HOME/.hushlogin"
mkdir -p "$DEMO_HOME/.zsh_sessions"

# ----------------------------------------------------------- the project ----
cd "$PROJECT"
export GIT_CONFIG_GLOBAL="$DEMO_HOME/.gitconfig"

commit() {  # commit <hours ago> <author> <message>
  local when; when="$(date -v-"$1"H '+%Y-%m-%d %H:%M:%S')"
  git add -A
  GIT_AUTHOR_NAME="$2" GIT_AUTHOR_EMAIL="$(echo "$2" | tr 'A-Z ' 'a-z.')@example.local" \
  GIT_COMMITTER_NAME="$2" GIT_COMMITTER_EMAIL="$(echo "$2" | tr 'A-Z ' 'a-z.')@example.local" \
  GIT_AUTHOR_DATE="$when" GIT_COMMITTER_DATE="$when" \
    git commit -q -m "$3"
}

git init -q

cat > package.json <<'JSON'
{
  "name": "acme-web",
  "version": "1.4.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "node --watch src/server.js",
    "test": "node --test"
  }
}
JSON
cat > .nvmrc <<<"22"
cat > README.md <<'MD'
# acme-web

The storefront for Acme Outdoor Supply.
MD
cat > .gitignore <<<"node_modules/"
commit 96 "Jordan Lee" "Start the storefront"

mkdir -p src test public
cat > src/server.js <<'JS'
import { createServer } from "node:http";
import { listProducts } from "./products.js";

const port = process.env.PORT ?? 3000;

createServer((req, res) => {
  if (req.url === "/api/products") {
    res.setHeader("content-type", "application/json");
    res.end(JSON.stringify(listProducts()));
    return;
  }
  res.statusCode = 404;
  res.end("Not found");
}).listen(port, () => console.log(`acme-web listening on http://localhost:${port}`));
JS
cat > src/products.js <<'JS'
const products = [
  { sku: "TNT-200", name: "Ridgeline 2P Tent", price: 249 },
  { sku: "PCK-045", name: "Summit 45L Pack", price: 189 },
  { sku: "BTL-010", name: "Trail Bottle 1L", price: 24 },
];

export function listProducts() {
  return products;
}

export function findProduct(sku) {
  return products.find((product) => product.sku === sku);
}
JS
commit 80 "Jordan Lee" "Serve the product list"

cat > src/cart.js <<'JS'
import { findProduct } from "./products.js";

export function cartTotal(items) {
  return items.reduce((total, { sku, qty }) => total + findProduct(sku).price * qty, 0);
}

export function applyDiscount(total, percent) {
  return Math.round(total * (100 - percent)) / 100;
}
JS
commit 60 "Casey Morgan" "Add the cart total"

cat > test/cart.test.js <<'JS'
import { test } from "node:test";
import assert from "node:assert/strict";
import { applyDiscount, cartTotal } from "../src/cart.js";
import { findProduct } from "../src/products.js";

test("adds up the cart", () => {
  assert.equal(cartTotal([{ sku: "TNT-200", qty: 1 }, { sku: "BTL-010", qty: 2 }]), 297);
});

test("takes a discount off", () => {
  assert.equal(applyDiscount(200, 15), 170);
});

test("finds a product by SKU", () => {
  assert.equal(findProduct("PCK-045").name, "Summit 45L Pack");
});

test("returns nothing for an unknown SKU", () => {
  assert.equal(findProduct("NOPE-000"), undefined);
});
JS
commit 40 "Riley Park" "Test the cart and product lookup"

cat > public/index.html <<'HTML'
<!doctype html>
<title>Acme Outdoor Supply</title>
<h1>Gear for the long way round</h1>
HTML
commit 20 "Alex Rivers" "Add the landing page"

git tag v1.4.0
git checkout -q -b feature/checkout

cat >> src/cart.js <<'JS'

export function shippingFor(total) {
  return total >= 100 ? 0 : 8;
}
JS
commit 6 "Alex Rivers" "Free shipping over \$100"

# Work in progress, so the changes chip has something to count.
cat > src/checkout.js <<'JS'
import { cartTotal, shippingFor } from "./cart.js";

export function checkout(items) {
  const subtotal = cartTotal(items);
  return { subtotal, shipping: shippingFor(subtotal), total: subtotal + shippingFor(subtotal) };
}
JS
sed -i '' 's/Gear for the long way round/Gear for the long way round. Free shipping over $100./' public/index.html
cat > access.log <<'LOG'
GET /api/products 200 4ms
GET /api/products 200 3ms
GET /favicon.ico 404 1ms
LOG
echo "access.log" >> .gitignore
