echo -e "\n packaging self \n"

# build binary
cargo build --release 

# move to more reasonable path
if [ ! -d "out" ]; then
  mkdir out
fi
cp target/release/self-vm out/self

# strip binary symbols
strip out/self

echo -e "
┌────────────────────────────────┐
│  📦 self packaged at out/self  │
│     bye, friend.               │
└────────────────────────────────┘
"

