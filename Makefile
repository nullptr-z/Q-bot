
run:
	@RUST_LOG=info cargo run

watch:
	@echo "watch.."
	@watchexec --restart --exts rs,js,css,jinja --ignore public -- make run

build-css:
	@echo "Building css.."
	@npx tailwindcss build -i html-ui/global.css -o html-ui/public/css/index.css  --watch
