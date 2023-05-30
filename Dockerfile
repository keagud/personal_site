FROM  python:3.11-bullseye
WORKDIR /app
COPY . /app
RUN git submodule update --init --recursive && rm -rf .git
RUN pip install -r requirements.txt && pip install gunicorn
RUN ./manage.py collectstatic
EXPOSE 8000
CMD ["gunicorn",  "personal_site.wsgi:application", "-b", "0.0.0.0:8000"]
