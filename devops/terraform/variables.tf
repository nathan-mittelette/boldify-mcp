variable "project" {
  type = object({
    name        = string
    environment = string
    provider    = string
    responsible = string
  })
}

variable "api_gateway" {
  type = object({
    cors = object({
      allow_origins = set(string)
      allow_headers = set(string)
      allow_methods = set(string)
    }),
    log = object({
      retention = number
    })
  })
}

variable "dns" {
  type = object({
    root_domain = string
    domain_name = string
  })
}

variable "lambdas" {
  type = set(object({
    name        = string
    output      = string
    source      = string
    description = string
    memory      = string
    timeout     = string
    runtime     = string
    handler     = string
    https = list(object({
      method = string
      path   = string
    }))
    layers       = optional(list(string), [])
    environments = optional(map(string), {})
  }))
}
