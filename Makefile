MANAGE := poetry run python manage.py

serve:
	$(MANAGE) runserver

shell:
	$(MANAGE) shell

format:
	( prettier ./**/templates/*/*  -u --write  > /dev/null & ) 
	(  isort  . -q  &&  black . -q    ) & 


test:
	$(MANAGE) test 
