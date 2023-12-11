
run: build-css
	@RUST_LOG=info proxychains4 cargo run

watch:
	@echo "watch.."
	@watchexec --restart --exts rs,js,css,jinja --ignore public -- make run

build-css:
	@echo "Building css.."
	@proxychains4 npx tailwindcss build -i html-ui/global.css -o html-ui/public/css/index.css

ui-start:
	@yarn --cwd app-ui dev -p 8081

