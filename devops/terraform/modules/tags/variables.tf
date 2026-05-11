variable "project" {
  type = object({
    name        = string,
    environment = string,
    provider    = string,
    responsible = string
  })
}
