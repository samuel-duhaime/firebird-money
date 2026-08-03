import type { en } from './en';

export const fr: typeof en = {
  nav: {
    dashboard: 'Tableau de bord',
    transactions: 'Transactions',
    rules: 'Règles',
  },
  leftMenu: {
    username: "Nom d'utilisateur",
    language: 'Langue',
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
    date: 'Date',
    category: 'Catégorie',
    selectCategory: 'Sélectionnez une catégorie',
    submit: 'Ajouter la transaction',
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
    downloadFailed: 'Échec du téléchargement des transactions.',
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
