locals {
  resources_name = "${lower(var.project.name)}-${lower(var.project.environment)}"
}
