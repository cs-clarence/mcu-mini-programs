pub mod response {
  use std::{borrow::Cow, fmt::Display};

  use axum::{
    body::Body,
    http::{Response, StatusCode},
    response::IntoResponse,
    Json,
  };
  use serde::Serialize;

  use crate::util::result;

  #[derive(Serialize, Debug)]
  pub struct ErrorResponse {
    message: String,
  }

  impl Display for ErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.message)
    }
  }

  // impl<T: Error> From<T> for ServerErrorResponse {
  //   fn from(value: T) -> Self {
  //     ServerErrorResponse::new(Cow::Owned(value.to_string()))
  //   }
  // }

  impl From<result::Error> for ErrorResponse {
    fn from(value: result::Error) -> Self {
      ErrorResponse::new(Cow::Owned(value.to_string()))
    }
  }

  impl ErrorResponse {
    pub fn new(message: impl Into<String>) -> Self {
      Self {
        message: message.into(),
      }
    }
  }

  impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response<Body> {
      let mut response = Json(self).into_response();

      *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;

      response
    }
  }

  pub fn server_error(message: &str) -> ErrorResponse {
    ErrorResponse::new(message)
  }

  #[derive(Serialize)]
  pub struct DataReponse<T> {
    data: T,
  }

  impl<T> DataReponse<T> {
    pub fn new(data: T) -> Self {
      Self { data }
    }
  }

  pub fn data<T>(data: T) -> DataReponse<T> {
    DataReponse::new(data)
  }

  impl<T> IntoResponse for DataReponse<T>
  where
    T: Serialize,
  {
    fn into_response(self) -> Response<Body> {
      Json(self).into_response()
    }
  }

  #[derive(Serialize)]
  pub struct SuccessResponse {
    message: String,
  }

  impl SuccessResponse {
    pub fn new(message: impl Into<String>) -> Self {
      Self {
        message: message.into(),
      }
    }
  }

  impl IntoResponse for SuccessResponse {
    fn into_response(self) -> Response<Body> {
      let mut response = Json(self).into_response();

      *response.status_mut() = StatusCode::OK;

      response
    }
  }

  pub fn success(message: &str) -> SuccessResponse {
    SuccessResponse::new(Cow::Borrowed(message))
  }
}
