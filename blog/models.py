from django.db import models

# Create your models here.


class PostModel(models.Model):
    content_file = models.FileField(upload_to="")
