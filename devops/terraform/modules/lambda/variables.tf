variable "project" {
  type = object({
    name        = string,
    environment = string,
    provider    = string,
    responsible = string
  })
}

variable "lambda" {
  type = object({
    name        = string,
    output      = string
    source      = string
    description = string
    memory      = string
    timeout     = string
    runtime     = string
    handler     = string
    http = object({
      method = string
      path   = string
    })
  })
}

variable "api_gateway" {
  type = object({
    execution_arn = string
    id            = string
  })
}
