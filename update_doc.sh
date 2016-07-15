set -e

cargo doc
FROM=$(git rev-parse --short HEAD)
git checkout gh-pages
git fetch && git rebase origin/gh-pages
rsync -a target/doc/ .
git add .
git commit -m "Update doc for ${FROM}"
# git push
git checkout master
