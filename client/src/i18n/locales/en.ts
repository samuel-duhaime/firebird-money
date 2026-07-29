export const en = {
  landing: {
    headline: 'Family finances, finally clear.',
    description:
      "Firebird brings your household's transactions, budgets, and spending rules into one place, so you can see where your money goes.",
    signIn: 'Sign in with email',
  },
  auth: {
    signIn: {
      title: 'Sign in',
      description:
        "Enter your email and we'll send you a link that signs you in. No password to remember.",
      placeholder: 'you@example.com',
      submit: 'Send me a link',
      sending: 'Sending…',
    },
    checkInbox: {
      title: 'Check your inbox',
      description:
        'We sent a sign-in link to {{email}}. It expires in 15 minutes.',
      useAnotherEmail: 'Use another email',
    },
    verify: {
      title: 'Signing you in…',
      failedTitle: 'This link no longer works',
      failedDescription:
        'Sign-in links expire quickly and can only be used once. Ask for a fresh one.',
      tryAgain: 'Back to sign in',
    },
  },
  onboarding: {
    title: 'Set up your household',
    description:
      "A household is where your family's transactions live. Start one, or join the one someone already set up.",
    create: {
      submit: 'Create a household',
    },
    join: {
      trigger: 'I have a join code',
      placeholder: 'Join code',
      submit: 'Join household',
    },
    back: 'Back',
  },
  nav: {
    dashboard: 'Dashboard',
    transactions: 'Transactions',
    rules: 'Rules',
  },
  leftMenu: {
    username: 'Username',
    language: 'Language',
  },
  dashboard: {
    loading: 'Loading categories…',
    error: 'Failed to load categories.',
    loaded_one: '{{count}} category loaded from the API.',
    loaded_other: '{{count}} categories loaded from the API.',
  },
  transactions: {
    topMenu: {
      clear: 'Clear',
      filters: 'Filters',
      add: 'Add',
    },

  add: {
    title: 'Add transaction',
    amount: 'Amount',
    merchant: 'Merchant',
    date: 'Date',
    category: 'Category',
    submit: 'Add transaction',
    selectCategory: 'Select category',
  },

    toolbar: {
      allTransactions: 'All transactions',
      editMultiple: 'Edit multiple',
      columns: 'Columns',
    },
    sort: {
      trigger: 'Sort',
      options: {
        date: 'Date (new to old)',
        inverse_date: 'Date (old to new)',
        amount: 'Amount (high to low)',
        inverse_amount: 'Amount (low to high)',
      },
    },
    search: {
      trigger: 'Search',
      title: 'Search',
      placeholder: 'Enter a search term...',
      help: "We'll match your search term to merchant names, categories, and amounts.",
      clear: 'Clear',
      cancel: 'Cancel',
      apply: 'Apply',
    },
    dateRange: {
      trigger: 'Date',
      title: 'Date Range',
      startDate: 'Start date',
      endDate: 'End date',
      placeholder: 'YYYY-MM-DD',
      pickA: 'Pick a {{label}}',
      clear: 'Clear',
      cancel: 'Cancel',
      apply: 'Apply',
      from: 'From {{date}}',
      until: 'Until {{date}}',
      presets: {
        last_7_days: 'Last 7 days',
        last_30_days: 'Last 30 days',
        this_month: 'This month',
        last_month: 'Last month',
        this_year: 'This year',
        last_year: 'Last year',
      },
    },
    download: {
      trigger: 'Download',
      csv: 'Download as CSV',
      xlsx: 'Download as Excel',
    },
    import: {
      trigger: 'Import',
      uploading: 'Uploading…',
      importing: 'Importing…',
    },
    list: {
      loading: 'Loading transactions…',
      error: 'Failed to load transactions.',
      empty: 'No transactions yet.',
    },
  },
  toast: {
    notImplemented: 'This feature is not available yet.',
    signInFailed: 'Could not send the sign-in link. Please try again.',
    onboardingFailed: 'Could not set up your household. Please try again.',
    joinCodeNotFound: 'No household matches this join code.',
    downloadFailed: 'Failed to download transactions.',
    importStarted: 'Import started — this can take a minute.',
    importFailed: 'Failed to import transactions.',
    importSucceeded_one: 'Imported {{count}} transaction.',
    importSucceeded_other: 'Imported {{count}} transactions.',
    importPartial_one: 'Imported {{count}} transaction ({{issues}}).',
    importPartial_other: 'Imported {{count}} transactions ({{issues}}).',
    importFailedCount_one: '{{count}} failed',
    importFailedCount_other: '{{count}} failed',
    importSkippedCount_one: '{{count}} skipped',
    importSkippedCount_other: '{{count}} skipped',
  },
};
