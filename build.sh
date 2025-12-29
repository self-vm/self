set -euo pipefail
echo -e "\n packaging self \n"

cargo build --release 

mkdir -p out
cp target/release/selfcli out/self
strip out/self

echo -e "

 📦 self packaged at out/self

┌───────┐────────────────────────┐
│  run  │                        │ 
│───────┘                        │
│  $ ./out/self ping             │
└────────────────────────────────┘
"

