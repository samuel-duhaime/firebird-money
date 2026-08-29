import type { en } from './en';

export const fr: typeof en = {
  landing: {
    headline: 'Tes finances familiales, enfin simples.',
    description:
      'Firebird regroupe les transactions, les budgets et les règles de dépenses de ton ménage au même endroit, pour que tu voies où va ton argent.',
    signIn: 'Se connecter par courriel',
  },
  auth: {
    signIn: {
      title: 'Connexion',
      description:
        'Entre ton courriel et on t’envoie un lien qui te connecte. Aucun mot de passe à retenir.',
      placeholder: 'toi@exemple.com',
      submit: 'Envoie-moi un lien',
      sending: 'Envoi…',
    },
    checkInbox: {
      title: 'Vérifie tes courriels',
      description:
        'On a envoyé un lien de connexion à {{email}}. Il expire dans 15 minutes.',
      useAnotherEmail: 'Utiliser un autre courriel',
    },
    verify: {
      title: 'Connexion en cours…',
      failedTitle: 'Ce lien ne fonctionne plus',
      failedDescription:
        'Les liens de connexion expirent rapidement et ne servent qu’une fois. Demandes-en un nouveau.',
      tryAgain: 'Retour à la connexion',
    },
  },
  onboarding: {
    title: 'Configure ton ménage',
    description:
      'Un ménage, c’est là que vivent les transactions de ta famille. Crées-en un, ou rejoins celui que quelqu’un a déjà créé.',
    create: {
      submit: 'Créer un ménage',
    },
    join: {
      trigger: 'J’ai un code d’invitation',
      placeholder: 'Code d’invitation',
      submit: 'Rejoindre le ménage',
    },
    back: 'Retour',
  },
  nav: {
    dashboard: 'Tableau de bord',
    transactions: 'Transactions',
    rules: 'Règles',
  },
  leftMenu: {
    username: "Nom d'utilisateur",
    language: 'Langue',
    settings: 'Paramètres',
    signOut: 'Se déconnecter',
  },
  dashboard: {
    loading: 'Chargement des catégories…',
    error: 'Échec du chargement des catégories.',
    loaded_one: '{{count}} catégorie chargée depuis l’API.',
    loaded_other: '{{count}} catégories chargées depuis l’API.',
  },
  transactions: {
    topMenu: {
      clear: 'Effacer',
      filters: 'Filtres',
      add: 'Ajouter',
    },
  
  add: {
    title: 'Ajouter une transaction',
    amount: 'Montant',
    merchant: 'Commerçant',
    merchantPlaceholder: 'Nom du commerçant',
    date: 'Date',
    category: 'Catégorie',
    selectCategory: 'Sélectionnez une catégorie',
    submit: 'Ajouter la transaction',
    cancel: 'Annuler',
    close: 'Fermer',
    required: 'Tous les champs sont obligatoires.',
    invalidAmount:
      'Entre un montant simple, par exemple 12,50, sans séparateur de milliers.',
    failed: "Impossible d'ajouter la transaction. Réessaie.",
  },

    toolbar: {
      allTransactions: 'Toutes les transactions',
      editMultiple: 'Modifier plusieurs',
      columns: 'Colonnes',
    },
    sort: {
      trigger: 'Trier',
      options: {
        date: 'Date (récent à ancien)',
        inverse_date: 'Date (ancien à récent)',
        amount: 'Montant (élevé à faible)',
        inverse_amount: 'Montant (faible à élevé)',
      },
    },
    search: {
      trigger: 'Rechercher',
      title: 'Rechercher',
      placeholder: 'Entrez un terme de recherche...',
      help: 'On va comparer ton terme de recherche aux noms de marchands, aux catégories et aux montants.',
      clear: 'Effacer',
      cancel: 'Annuler',
      apply: 'Appliquer',
    },
    dateRange: {
      trigger: 'Date',
      title: 'Plage de dates',
      startDate: 'Date de début',
      endDate: 'Date de fin',
      placeholder: 'AAAA-MM-JJ',
      pickA: 'Choisir une {{label}}',
      clear: 'Effacer',
      cancel: 'Annuler',
      apply: 'Appliquer',
      from: 'À partir du {{date}}',
      until: "Jusqu'au {{date}}",
      presets: {
        last_7_days: '7 derniers jours',
        last_30_days: '30 derniers jours',
        this_month: 'Ce mois-ci',
        last_month: 'Mois dernier',
        this_year: 'Cette année',
        last_year: 'Année dernière',
      },
    },
    download: {
      trigger: 'Télécharger',
      csv: 'Télécharger en CSV',
      xlsx: 'Télécharger en Excel',
    },
    import: {
      trigger: 'Importer',
      uploading: 'Téléversement…',
      importing: 'Importation…',
    },
    list: {
      loading: 'Chargement des transactions…',
      error: 'Échec du chargement des transactions.',
      empty: 'Aucune transaction pour le moment.',
    },
  },
  toast: {
    notImplemented: "Cette fonctionnalité n'est pas encore disponible.",
    signInFailed: 'Impossible d’envoyer le lien de connexion. Réessaie.',
    signOutFailed: 'Impossible de te déconnecter. Réessaie.',
    onboardingFailed: 'Impossible de configurer ton ménage. Réessaie.',
    joinCodeNotFound: 'Aucun ménage ne correspond à ce code d’invitation.',
    downloadFailed: 'Échec du téléchargement des transactions.',
    addTransactionSucceeded: 'Transaction ajoutée.',
    importStarted: 'Importation commencée — ça peut prendre une minute.',
    importFailed: "Échec de l'importation des transactions.",
    importSucceeded_one: '{{count}} transaction importée.',
    importSucceeded_other: '{{count}} transactions importées.',
    importPartial_one: '{{count}} transaction importée ({{issues}}).',
    importPartial_other: '{{count}} transactions importées ({{issues}}).',
    importFailedCount_one: '{{count}} échec',
    importFailedCount_other: '{{count}} échecs',
    importSkippedCount_one: '{{count}} ignorée',
    importSkippedCount_other: '{{count}} ignorées',
  },
};
