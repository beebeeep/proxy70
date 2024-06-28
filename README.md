proxy70
=======
Simple and clean HTTP proxy for browsing gopherspace via your browser.

Text-based gopher browsers are nice, but I was utterly annoyed that they tend to reload
resource every time you go back — and believe it or not, gopher holes are not always fast to respond. Web browsers, meantime, used to work with slow sites, so they have caches, which
makes browsing experience way faster. Oh, and also browsers can show images. 

Features
========
Proxy will inline images, sound files (as long as you browser supports whatever format is there), query prompts. Text files will be shown in browser (press "w" to toggle line wrapping), any other files will be simply downloaded. 

Directory view preserve ASCII art and support 4/8/24 bit color via ANSI escape codes. 

Installation and usage
======================
Checkout repo, run `cargo run` and open http://localhost:8080