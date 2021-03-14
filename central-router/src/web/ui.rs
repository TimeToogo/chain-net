use warp::{Filter, Reply, filters::BoxedFilter, hyper::Response, path};

#[cfg(not(dev))]
pub fn get() -> BoxedFilter<(impl Reply,)> {
    let index = include_str!("static/index.html");
    let css = include_str!("static/style.css");
    let js = include_str!("static/app.js");

    warp::get()
        .and(
            path::end()
                .map(move || {
                    Response::builder()
                        .header("Content-Type", "text/html")
                        .body(index)
                })
                .or(path("style.css").map(move || {
                    Response::builder()
                        .header("Content-Type", "text/css")
                        .body(css)
                }))
                .or(path("app.js").map(move || {
                    Response::builder()
                        .header("Content-Type", "text/javascript")
                        .body(js)
                })),
        )
        .boxed()
}

#[cfg(dev)]
pub fn get() -> BoxedFilter<(impl Reply,)> {
    warp::fs::dir("central-router/src/web/static").boxed()
}
