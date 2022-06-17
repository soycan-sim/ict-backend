# ict-backend

This project won't be seeing any major updates, but I'm keeping it up for demonstration purposes.

This is the backend for an e-commerce website. It uses Postgres (via [tokio-postgres](https://crates.io/crates/tokio-postgres)) for persistent data,
HTML5 and vanilla JS with [a custom template engine](./src/template.rs) inspired by [Handlebars](https://handlebarsjs.com/) for the frontend,
and markdown (via [pulldown-cmark](https://crates.io/crates/pulldown-cmark)) for content.

The template engine also handles internationalization.

This backend uses async io for all of its io operations ([tokio](https://crates.io/crates/tokio)) and [actix-web](https://crates.io/crates/actix-web)
for establishing and maintaining connections. The backend runs both HTTP and HTTPS simultaneously.

Payments are handled with the [PayPal API](https://developer.paypal.com/home).

# License

Everything available in this repository is licensed under the [MIT license](./LICENSE).
