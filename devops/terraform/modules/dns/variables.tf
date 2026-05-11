variable "dns" {
  type = object({
    root_domain = string,
    domain_name = string,
  })
}
