# Chaînes visibles de l’API

transaction-not-found = Aucune transaction avec l’id { $n }
category-not-found = Aucune catégorie avec l’id { $n }
category-duplicate-name = Une catégorie avec ce nom existe déjà
category-in-use = La catégorie { $n } est toujours utilisée par des transactions existantes
category-group-not-found = Aucun groupe de catégories avec l’id { $n }
category-group-duplicate-name = Un groupe de catégories avec ce nom existe déjà
category-group-invalid-type = type doit être income, expense ou transfer
category-group-in-use = Le groupe de catégories { $n } est toujours utilisé par des catégories existantes
household-not-found = Aucun ménage avec l’id { $n }
household-in-use = Le ménage { $n } a encore des données qui y sont connectées (membres, groupes de catégories, catégories ou transactions)
user-not-found = Aucun utilisateur avec l’id { $n }
user-duplicate-email = Un utilisateur avec ce courriel existe déjà
user-invalid-status = status doit être verified, pending ou suspended
user-in-use = L’utilisateur { $n } est toujours connecté à un ménage
household-member-not-found = Aucun membre de ménage avec l’id { $n }
household-member-duplicate = Cet utilisateur est déjà connecté à un ménage
household-member-invalid-type = type doit être family_manager ou family_member
# Courriel de connexion. Le lien est inséré entre les instructions et la signature.
auth-email-subject = Ton lien de connexion FireBird Money
auth-email-greeting = Bonjour,
auth-email-instructions = Clique sur le lien ci-dessous pour te connecter à FireBird Money. Il expire dans 15 minutes et ne sert qu’une fois.
auth-email-ignore = Si tu n’as pas demandé à te connecter, tu peux ignorer ce courriel.

auth-email-invalid = Une adresse courriel valide est requise
auth-token-invalid = Ce lien de connexion est invalide, expiré ou déjà utilisé
auth-email-send-failed = Le courriel de connexion n’a pas pu être envoyé, veuillez réessayer
auth-not-signed-in = Vous devez être connecté
auth-no-household = Vous devez appartenir à un ménage
auth-join-code-not-found = Aucun ménage ne correspond à ce code d’invitation
auth-join-code-blank = join_code ne peut pas être vide; omets-le pour créer un nouveau ménage à la place
auth-already-in-household = Vous êtes déjà connecté à un ménage
import-job-not-found = Aucune tâche d’importation avec cet identifiant
import-file-required = Un fichier à importer est requis
import-file-too-large = Le fichier téléversé est trop volumineux (10 Mo maximum)
internal-db-error = Erreur interne du serveur
