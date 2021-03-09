use serde::{Deserialize, Serialize};
use warp::{filters::BoxedFilter, hyper::StatusCode, Filter, Reply};

use crate::state::SharedState;

#[derive(Serialize)]
struct Status {
    on: bool,
}

#[derive(Deserialize)]
struct NewStatus {
    on: bool,
}

pub fn get(state: SharedState) -> BoxedFilter<(impl Reply,)> {
    warp::get()
        .map(move || {
            warp::reply::json(&Status {
                on: state.get(|s| s.on)
            })
        })
        .boxed()
}

pub fn post(state: SharedState) -> BoxedFilter<(impl Reply,)> {
    warp::post()
        .and(warp::body::json())
        .map(move |n: NewStatus| {
            state.update(move |s| {
                s.on = n.on;
            });

            StatusCode::OK
        })
        .boxed()
}
