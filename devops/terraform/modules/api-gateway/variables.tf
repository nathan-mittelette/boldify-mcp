variable "project" {
  type = object({
    name        = string,
    environment = string,
    provider    = string,
    responsible = string
  })
}

variable "cors" {
  type = object({
    allow_origins = set(string)
    allow_headers = set(string)
    allow_methods = set(string)
  })
}

variable "log" {
  type = object({
    retention = number
  })
}

variable "dns" {
  type = object({
    root_domain = string
    domain_name = string,
  })
}
