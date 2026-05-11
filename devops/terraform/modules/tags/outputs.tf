output "map" {
  value = {
    project     = var.project.name
    environment = var.project.environment
    provider    = var.project.provider
    responsible = var.project.responsible
  }
}
